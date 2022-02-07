use std::collections::HashMap;
use std::process::abort;
use std::sync::{Arc, Mutex, mpsc, mpsc::{Sender}};
use std::time::Duration;
use serde_json;
use serde::Serialize;
use std::process::{Command, Stdio, ChildStdin, ChildStdout};
use std::io::{Read, Write};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Usage: {} <source.bib> <output.json>", args[0]);
        abort();
    }
    let contents = leak_memory_of_string_into_static(std::fs::read_to_string(&args[1]).unwrap());
    let results: Arc<Mutex<Vec<CompletedEntry>>> = Arc::new(Mutex::new(vec![]));
    let mut finalize_pool: Vec<Sender<Entry>> = vec![];
    let mut thread_handles = vec![];
    //create the thread pool for parsing the actual entries 
    for _ in 0..num_cpus::get() {
        let (tx, rx) = mpsc::channel::<Entry>();
        let results_clone = results.clone();
        finalize_pool.push(tx);
        let handle = std::thread::spawn(move || {
            //spawn a python process
            let python_interpreter = Command::new("python3")
                .args(vec!["-c", "from pylatexenc import latex2text\nimport json\ndecoder=latex2text.LatexNodes2Text().latex_to_text\nwhile(True):\n try:\n  text=input()\n except EOFError:\n  exit(0)\n text=json.loads(text)\n res=decoder(text)\n print(json.dumps(res))"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn().unwrap();
            let mut stdin = python_interpreter.stdin.unwrap();
            let mut stdout = python_interpreter.stdout.unwrap();
            loop {
                let entry = rx.recv();
                if entry.is_err() {
                    return;
                }
                let entry = entry.unwrap();
                let completed_entry = entry.finalize(contents, &mut stdin, &mut stdout);
                results_clone.lock().unwrap().push(completed_entry);
            }
        });
        thread_handles.push(handle);
    }
    let num_entries = automaton_for_reading(&contents, &finalize_pool);
    while num_entries!=results.lock().unwrap().len() {
        std::thread::sleep(Duration::from_secs(1));
    }
    let temp = results.lock().unwrap();
    let res: &Vec<CompletedEntry> = temp.as_ref();
    std::fs::write(&args[2], serde_json::to_string(res).unwrap()).unwrap();
}

#[derive(Debug)]
struct Mark{
    pub start: usize,
    pub end_exclusice: usize
}

impl Mark{
    unsafe fn extract(self, input :&str) -> &str {
        input.get_unchecked(self.start..self.end_exclusice)
    }
}

#[derive(Debug)]
struct Entry{
    pub original: Mark,
    pub entry_type: Mark, 
    pub bibkey: Mark, 
    pub fields: Vec<(Mark, Mark, bool)>
}


#[derive(Debug, Serialize)]
struct CompletedEntry<'a> {
    pub original: &'a str,
    pub entry_type: &'a str,
    pub bibkey: &'a str,
    pub fields: HashMap<&'a str, &'a str>
}

impl Entry{
    fn new(start: usize) -> Entry {
        Entry { 
            original: Mark{start: start, end_exclusice: start}, 
            entry_type: Mark{start: start, end_exclusice: start}, 
            bibkey: Mark{start: start, end_exclusice: start}, 
            fields: vec![]
        }
    }
    fn finalize<'a>(self, input: &'a str, stdin: &mut ChildStdin, stdout: &mut ChildStdout) -> CompletedEntry<'a> {
        unsafe {
            let mut hm = HashMap::new();
            for field in self.fields {
                let name = field.0.extract(input);
                let mut value = field.1.extract(input);
                if field.2 == true {
                    stdin.write(serde_json::to_string(value).unwrap().as_bytes()).unwrap();
                    stdin.write(&[b'\n']).unwrap();
                    let mut buffer = vec![0;2000];
                    let mut decoded_value = "".to_string();
                    loop {
                        let bytes_read = stdout.read(&mut buffer).unwrap();
                        decoded_value.push_str(&String::from_utf8(buffer[0..bytes_read].to_vec()).unwrap());
                        if buffer[0..bytes_read].contains(&b'\n'){
                            break;
                        }
                    }
                    let decoded_value = serde_json::from_str(&decoded_value).unwrap();
                    value = leak_memory_of_string_into_static(decoded_value);
                } 
                hm.insert(name, value);
            }
            CompletedEntry{
                original: self.original.extract(input),
                entry_type: self.entry_type.extract(input),
                bibkey: self.bibkey.extract(input),
                fields: hm,
            }
        }
    }
}

fn leak_memory_of_string_into_static(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn automaton_for_reading<'a>(input_string: &'a str, finalize_pool: &Vec<Sender<Entry>>) -> usize{
    type S = ParsingStates;
    let input: Vec<char> = input_string.chars().into_iter().collect();
    let mut state = ParsingStates::SeekEntry;
    let mut entry = Entry::new(0);
    let mut current_field_name = Mark{start: 0, end_exclusice: 0};
    let mut current_field_value = Mark{start: 0, end_exclusice: 0};
    let mut open_brackets = 0;
    let mut current_field_value_latex = false;
    let mut current_pool_node = 0;
    let mut num_of_entries = 0;
    for i in 0..input.len() {
        match state {
            S::SeekEntry => {
                if input[i]=='@' {state = S::ReadType; entry=Entry::new(i); entry.entry_type.start=i+1;}
            }
            S::ReadType => {
                if input[i]=='{' {state = S::ReadBibkey; entry.entry_type.end_exclusice=i; entry.bibkey.start=i+1;}
            }
            S::ReadBibkey => {
                if input[i]==',' {state = S::SeekFieldName; entry.bibkey.end_exclusice=i;}
            }
            S::SeekFieldName => {
                if input[i].is_alphabetic() {state = S::ReadFieldName; current_field_name.start=i;}
            }
            S::ReadFieldName => {
                if input[i].is_whitespace() {state = S::SeekEqualsSign; current_field_name.end_exclusice=i;}
                else if input[i]=='=' {state = S::SeekFieldValueBracket; current_field_name.end_exclusice=i;}
            }
            S::SeekEqualsSign => {
                if input[i]=='=' {state = S::SeekFieldValueBracket;}
            }
            S::SeekFieldValueBracket => {
                if input[i] == '{' {state = S::ReadFieldValue; current_field_value.start=i+1;}
            }
            S::ReadFieldValue => {
                match input[i] {
                    '$' => {
                        current_field_value_latex = true;
                    }
                    '{' => {
                        open_brackets += 1;
                        current_field_value_latex = true;
                    }
                    '}' => {
                        if open_brackets!=0 {open_brackets-=1;
                            if open_brackets<0 {panic!();}
                        }
                        else if open_brackets==0 {
                            state = S::DoneReadingFieldValue; 
                            current_field_value.end_exclusice=i;
                            entry.fields.push((current_field_name, current_field_value, current_field_value_latex));
                            current_field_name = Mark{start: 0, end_exclusice: 0};
                            current_field_value = Mark{start: 0, end_exclusice: 0};
                            current_field_value_latex = false;
                        }
                    }
                    '\\' => {
                        state = S::ReadFieldValueEscape;
                        current_field_value_latex = true;
                    }
                    _ => {}
                }
            }
            S::ReadFieldValueEscape => {
                state = S::ReadFieldValue;
            }
            S::DoneReadingFieldValue => {
                if input[i]==',' {state = S::SeekFieldName;}
                else if input[i]=='}' {
                    state = S::SeekEntry;
                    entry.original.end_exclusice = i+1;
                    //we want to use all cpu, thus we split the work for actually parsing accross the pool
                    finalize_pool[current_pool_node].send(entry).unwrap();
                    current_pool_node = (current_pool_node +1) % finalize_pool.len();
                    num_of_entries += 1;
                    entry = Entry::new(0);
                }
            }
        }
    }
    return num_of_entries;
}


#[derive(Debug)]
enum ParsingStates{
    SeekEntry,
    ReadType,
    ReadBibkey,
    SeekFieldName,
    ReadFieldName,
    SeekEqualsSign,
    SeekFieldValueBracket,
    ReadFieldValue,
    ReadFieldValueEscape,
    DoneReadingFieldValue
}
