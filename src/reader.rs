use rayon::prelude::*;

use crate::models::{TypeLine, TypeLineCounter, TypeLineResults};
use crate::printer;
use std::{
    borrow::Cow,
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    time::Instant,
};
use std::{
    borrow::{Borrow, BorrowMut},
    io::Read,
    sync::{Arc, Mutex},
};

const ERROR_TYPE: &'static str = "ERROR";

// TYPE: A | TOTAL COUNT: 98514 | TOTAL BYTES: 4630158
// TYPE: B | TOTAL COUNT: 7488 | TOTAL BYTES: 357084
// TYPE: C | TOTAL COUNT: 68796 | TOTAL BYTES: 3233412
// TYPE: D | TOTAL COUNT: 163800 | TOTAL BYTES: 7698600
// Took 1358168 microseconds

// TYPE: A | TOTAL COUNT: 98514 | TOTAL BYTES: 4630158
// TYPE: B | TOTAL COUNT: 7488 | TOTAL BYTES: 357084
// TYPE: C | TOTAL COUNT: 68796 | TOTAL BYTES: 3233412
// TYPE: D | TOTAL COUNT: 163800 | TOTAL BYTES: 7698600
// Took 1421491 microseconds

pub fn start(path: PathBuf, pretty_print: bool) {
    let init = Instant::now();
    if let Ok(f) = File::open(&path) {
        let mut br = BufReader::new(f);
        let results = calculate_results(&mut br);
        // let results = calculate_results_par(f);
        // let results = results.lock().unwrap();
        // let results = results.borrow();
        printer::print_table(pretty_print, &results);
    } else {
        eprintln!("Error trying to open the file {:?}", path);
    }
    println!("Took {:?} microseconds", init.elapsed().as_micros());
}

const CHUNK_SIZE: usize = 1_000_000;

fn find_last_newline_position(buf: &[u8]) -> usize {
    let mut i = buf.len() - 1;
    while i > 0 {
        if buf[i] == b'\n' {
            return i + 1;
        }
        i -= 1;
    }
    buf.len()
}

fn calculate_results_par(mut f: File) -> Arc<Mutex<TypeLineResults<'static>>> {
    let results = Arc::new(Mutex::new(HashMap::new()));
    rayon::scope(|scope| {
        let mut buf = Vec::with_capacity(CHUNK_SIZE);
        loop {
            // read what we need
            f.by_ref()
                .take((CHUNK_SIZE - buf.len()) as u64)
                .read_to_end(&mut buf)
                .unwrap();

            // short circuit check
            if buf.len() == 0 {
                break;
            }

            // Copy any incomplete lines to the next s.
            let last_newline_position = find_last_newline_position(&buf);
            let mut next_buf = Vec::with_capacity(CHUNK_SIZE);
            next_buf.extend_from_slice(&buf[last_newline_position..]);
            buf.truncate(last_newline_position);

            // start rayon job
            let results_clone = results.clone();
            scope.spawn(move |_| {
                buf[..last_newline_position]
                    .split(|c| *c == b'\n')
                    .enumerate()
                    .par_bridge()
                    .for_each(|(line_number, line)| {
                        let num_bytes = line.len() + 1;
                        match serde_json::from_slice::<TypeLine>(line) {
                            Ok(typeline) => {
                                results_clone
                                    .lock()
                                    .unwrap()
                                    .borrow_mut()
                                    .entry(Cow::Owned(typeline.linetype))
                                    .or_insert(TypeLineCounter::default())
                                    .add_bytes(num_bytes);
                            }
                            Err(e) if num_bytes != 1 => {
                                eprintln!(
                                    "Error found parsing line {} - bytes {}: {:?}",
                                    line_number, num_bytes, e
                                );
                                results_clone
                                    .lock()
                                    .unwrap()
                                    .borrow_mut()
                                    .entry(Cow::Borrowed(ERROR_TYPE))
                                    .or_insert(TypeLineCounter::default())
                                    .add_bytes(num_bytes);
                            }
                            Err(_) => (), // end of file
                        }
                    });
            });
            buf = next_buf;
        }
    });
    results
}

/// NOTE: I chose to use a BufRead impl because I didn't want to have all the file in memory.
/// I chose the impl to allow me to pass a &[u8] from the tests while avoiding dynamic dispatching.
fn calculate_results(buffer_reader: &mut impl BufRead) -> TypeLineResults {
    let mut buf = String::new();
    let mut results = HashMap::new();
    let mut line_number = 1;

    loop {
        // using read_line instead of the lines iterator as this is slighly faster
        // it also includes the end line char and computes the number of bytes
        // I decided to panic in case there were issues with the encoding.
        // NOTE: I tried paralellizing the reading of the lines by using Rayon `par_bridge`
        // over the `lines` iterator but it was significantly slower.
        // Probably due to the mutex penalty I was unable to overcome.
        // I also tried to read the file by chunks and do the parsing in several rayon
        // spawned jobs but pretty much the same.
        let num_bytes = buffer_reader.read_line(&mut buf).expect("Not UTF-8 found");

        // I used serde in order to validate that the text is valid JSON
        // and used a simple struct which only cares about the `type` property.
        // In case bad formatted JSON I decided to go on and count the error as a new
        // category and also output the error in stderr.
        match serde_json::from_str::<TypeLine>(&buf) {
            Ok(typeline) => {
                results
                    .entry(Cow::Owned(typeline.linetype))
                    .or_insert(TypeLineCounter::default())
                    .add_bytes(num_bytes);
            }
            Err(e) if num_bytes != 0 => {
                eprintln!("Error found parsing line {}: {:?}", line_number, e);
                results
                    .entry(Cow::Borrowed(ERROR_TYPE))
                    .or_insert(TypeLineCounter::default())
                    .add_bytes(num_bytes);
            }
            Err(_) => (),
        }
        // clear buffer and update line number (used in case of error)
        buf.clear();
        line_number += 1;
        // short circuit check
        if num_bytes == 0 {
            break;
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_correctly_creates_the_sets() {
        let mut file_content = r#"{"type":"B","foo":"bar","items":["one","two"]}
{"type":"B","foo":"bar","items":["one","two"]}
{"type":"A","foo":"bar","items":["one","two"]}
{"type":"C","foo":"bar","items":["one","two"]}
"#
        .as_bytes();
        let result = calculate_results(&mut file_content);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn it_adds_empty_lines_as_errors_creating_the_sets() {
        let mut file_content = r#"{"type":"B","foo":"bar","items":["one","two"]}
{"type":"B","foo":"bar","items":["one","two"]}
{"type":"A","foo":"bar","items":["one","two"]}

{"type":"C","foo":"bar","items":["one","two"]}
"#
        .as_bytes();
        let result = calculate_results(&mut file_content);
        assert_eq!(result.len(), 4);
        assert!(result.get(ERROR_TYPE).is_some())
    }

    #[test]
    fn it_adds_bad_formatted_json_as_errors_creating_the_sets() {
        let mut file_content = r#"{"type":"B" "foo":"bar","items":["one","two"]}
{"type":"B","foo":"bar","items":["one","two"]}
{"type":"A","foo":"bar","items":["one","two"]}
{"type":"C","foo":"bar","items":["one","two"]}
"#
        .as_bytes();
        let result = calculate_results(&mut file_content);
        assert_eq!(result.len(), 4);
        assert!(result.get(ERROR_TYPE).is_some())
    }

    #[test]
    fn it_adds_json_with_no_type_as_errors_creating_the_sets() {
        let mut file_content = r#"{"type1":"B" "foo":"bar","items":["one","two"]}
{"type":"B","foo":"bar","items":["one","two"]}
{"type":"A","foo":"bar","items":["one","two"]}
{"type":"C","foo":"bar","items":["one","two"]}
"#
        .as_bytes();
        let result = calculate_results(&mut file_content);
        assert_eq!(result.len(), 4);
        assert!(result.get(ERROR_TYPE).is_some())
    }

    #[test]
    fn it_takes_into_account_spaces_when_counting_bytes() {
        let mut file_content =
            r#"  {  "type":"B", "foo":"bar","items":["one","two"]}  "#.as_bytes();
        let num_bytes = file_content.len();
        let result = calculate_results(&mut file_content);
        assert_eq!(result.len(), 1);
        assert!(result.get(ERROR_TYPE).is_none());
        assert_eq!(result.get("B").map(|r| r.bytes), Some(num_bytes));
    }

    #[test]
    fn it_takes_into_account_spaces_when_counting_bytes_even_when_invalid_json() {
        let mut file_content = r#"  {  "type":"B" "foo":"bar","items":["one","two"]}  "#.as_bytes();
        let num_bytes = file_content.len();
        let result = calculate_results(&mut file_content);
        let error = result.get(ERROR_TYPE).map(|r| r.bytes);
        assert_eq!(result.len(), 1);
        assert!(error.is_some());
        assert_eq!(error, Some(num_bytes));
    }
}
