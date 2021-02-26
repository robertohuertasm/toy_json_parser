use crate::models::{IntermediateTypeLineCounter, TypeLine, TypeLineCounter, TypeLineResults};
use crate::printer;
use std::io::Read;
use std::{
    borrow::Cow,
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    sync::mpsc::channel,
    thread::spawn,
    time::Instant,
};

const ERROR_TYPE: &'static str = "ERROR";

pub fn start(
    path: PathBuf,
    pretty_print: bool,
    use_chunks: bool,
    chunk_size: usize,
    verbose_errors: bool,
) {
    let init = Instant::now();
    if let Ok(f) = File::open(&path) {
        if use_chunks {
            let results = calculate_results(f, chunk_size, verbose_errors);
            printer::print_table(pretty_print, &results);
        } else {
            let mut br = BufReader::new(f);
            let results = calculate_results_naive(&mut br, verbose_errors);
            printer::print_table(pretty_print, &results);
        };
    } else {
        eprintln!("Error trying to open the file {:?}", path);
    }
    println!("Took {:?} microseconds", init.elapsed().as_micros());
}

fn find_last_newline_position(buf: &[u8]) -> Option<usize> {
    let mut i = buf.len() - 1;
    while i > 0 {
        if buf[i] == b'\n' {
            return Some(i + 1);
        }
        i -= 1;
    }
    None
}

fn calculate_results(
    mut f: impl Read,
    chunk_size: usize,
    verbose_errors: bool,
) -> TypeLineResults<'static> {
    let mut results = HashMap::new();
    let mut buf = Vec::with_capacity(chunk_size);
    let mut fatal_error = None;
    let (tx, rx) = channel();
    let mut threads = Vec::new();
    loop {
        // read what we need
        f.by_ref()
            .take((chunk_size - buf.len()) as u64)
            .read_to_end(&mut buf)
            .unwrap();

        // short circuit check
        if buf.len() == 0 {
            break;
        }

        // copy incomplete lines to the next buffer.
        if let Some(last_newline_position) = find_last_newline_position(&buf) {
            let mut next_buf = Vec::with_capacity(chunk_size);
            next_buf.extend_from_slice(&buf[last_newline_position..]);
            buf.truncate(last_newline_position);

            // start threads and capture the results
            let thread_tx = tx.clone();
            let thread_buf = buf;
            let thread = spawn(move || {
                let mut intermediate_counters = Vec::new();
                thread_buf[..last_newline_position]
                    .split(|c| *c == b'\n')
                    .into_iter()
                    .for_each(|line| {
                        let num_bytes = line.len() + 1; // adding the end line char
                        match serde_json::from_slice::<TypeLine>(line) {
                            Ok(typeline) => {
                                intermediate_counters.push(IntermediateTypeLineCounter {
                                    key: Cow::Owned(typeline.linetype),
                                    bytes: num_bytes,
                                });
                            }
                            Err(e) => {
                                if verbose_errors {
                                    eprintln!(
                                        "Error found parsing line: {} bytes - {:?}",
                                        num_bytes, e
                                    );
                                }

                                intermediate_counters.push(IntermediateTypeLineCounter {
                                    key: Cow::Borrowed(ERROR_TYPE),
                                    bytes: num_bytes,
                                });
                            }
                        }
                    });

                if let Err(e) = thread_tx.send(intermediate_counters) {
                    if verbose_errors {
                        eprintln!("{:?}", e);
                    }
                }
            });
            threads.push(thread);
            buf = next_buf;
        } else {
            fatal_error = Some(
                r#"FATAL ERROR: Either the chunk size is smaller than the lines you want to parse or your file doesn't end with a newline char."#,
            );
            break;
        }
    }

    if let Some(error) = fatal_error {
        eprintln!("{}", error);
    } else {
        let threads_len = threads.len();

        for t in threads {
            t.join().expect("The thread panicked");
        }
        for _ in 0..threads_len {
            match rx.recv() {
                Ok(intermediate_counters) => {
                    for ic in intermediate_counters {
                        results
                            .entry(ic.key)
                            .or_insert(TypeLineCounter::default())
                            .add_bytes(ic.bytes);
                    }
                }
                Err(e) => {
                    eprintln!("Something went wrong with the file reading {:?}", e);
                }
            }
        }

        // rectify the end of line error for each thread
        if let Some((key, mut counter)) = results.remove_entry(ERROR_TYPE) {
            counter.bytes -= threads_len;
            counter.count -= threads_len;
            if counter.bytes > 0 {
                results.insert(key, counter);
            }
        }
    }

    results
}

// NOTE: I chose to use a BufRead impl because I didn't want to have all the file in memory.
// I chose the impl to allow me to pass a &[u8] from the tests while avoiding dynamic dispatching.
fn calculate_results_naive(
    buffer_reader: &mut impl BufRead,
    verbose_errors: bool,
) -> TypeLineResults {
    let mut buf = String::new();
    let mut results = HashMap::new();
    let mut line_number = 1;

    loop {
        // using read_line instead of the lines iterator as this is slighly faster
        // it also includes the end line char and computes the number of bytes
        // I decided to panic in case there were issues with the encoding.
        // NOTE: I tried parallelizing the reading of the lines by using Rayon `par_bridge`
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
                if verbose_errors {
                    eprintln!("Error found parsing line {} - {:?}", line_number, e);
                }
                results
                    .entry(Cow::Borrowed(ERROR_TYPE))
                    .or_insert(TypeLineCounter::default())
                    .add_bytes(num_bytes);
            }
            Err(_) => (), // end of line
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
    fn calculate_results_correctly_creates_the_sets() {
        let mut file_content = r#"{"type":"B","foo":"bar","items":["one","two"]}
{"type":"B","foo":"bar","items":["one","two"]}
{"type":"A","foo":"bar","items":["one","two"]}
{"type":"C","foo":"bar","items":["one","two"]}
"#
        .as_bytes();
        let result = calculate_results(&mut file_content, 1_000, false);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn calculate_results_adds_empty_lines_as_errors_creating_the_sets() {
        let mut file_content = r#"{"type":"B","foo":"bar","items":["one","two"]}
{"type":"B","foo":"bar","items":["one","two"]}
{"type":"A","foo":"bar","items":["one","two"]}

{"type":"C","foo":"bar","items":["one","two"]}
"#
        .as_bytes();
        let result = calculate_results(&mut file_content, 1_000, false);
        assert_eq!(result.len(), 4);
        assert!(result.get(ERROR_TYPE).is_some())
    }

    #[test]
    fn calculate_results_adds_bad_formatted_json_as_errors_creating_the_sets() {
        let mut file_content = r#"{"type":"B" "foo":"bar","items":["one","two"]}
{"type":"B","foo":"bar","items":["one","two"]}
{"type":"A","foo":"bar","items":["one","two"]}
{"type":"C","foo":"bar","items":["one","two"]}
"#
        .as_bytes();
        let result = calculate_results(&mut file_content, 1_000, false);
        assert_eq!(result.len(), 4);
        assert!(result.get(ERROR_TYPE).is_some())
    }

    #[test]
    fn calculate_results_adds_json_with_no_type_as_errors_creating_the_sets() {
        let mut file_content = r#"{"type1":"B" "foo":"bar","items":["one","two"]}
{"type":"B","foo":"bar","items":["one","two"]}
{"type":"A","foo":"bar","items":["one","two"]}
{"type":"C","foo":"bar","items":["one","two"]}
"#
        .as_bytes();
        let result = calculate_results(&mut file_content, 1_000, false);
        assert_eq!(result.len(), 4);
        assert!(result.get(ERROR_TYPE).is_some())
    }

    #[test]
    fn calculate_results_takes_into_account_spaces_when_counting_bytes() {
        let mut file_content = r#"  {  "type":"B", "foo":"bar","items":["one","two"]}  
"#
        .as_bytes();
        let num_bytes = file_content.len();
        let result = calculate_results(&mut file_content, 1_000, false);
        assert_eq!(result.len(), 1);
        assert!(result.get(ERROR_TYPE).is_none());
        assert_eq!(result.get("B").map(|r| r.bytes), Some(num_bytes));
    }

    #[test]
    fn calculate_results_takes_into_account_spaces_when_counting_bytes_even_when_invalid_json() {
        let mut file_content = r#"  {  "type":"B" "foo":"bar","items":["one","two"]}  
"#
        .as_bytes();
        let num_bytes = file_content.len();
        let result = calculate_results(&mut file_content, 1_000, false);
        let error = result.get(ERROR_TYPE).map(|r| r.bytes);
        assert_eq!(result.len(), 1);
        assert!(error.is_some());
        assert_eq!(error, Some(num_bytes));
    }

    #[test]
    fn calculate_results_does_not_work_when_file_does_not_end_with_newline() {
        let mut file_content = r#"{ "type":"B", "foo":"bar","items":["one","two"]}"#.as_bytes();
        let result = calculate_results(&mut file_content, 1_000, false);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn calculate_results_does_not_work_when_the_chunks_are_smaller_than_a_line() {
        let mut file_content = r#"{ "type":"B", "foo":"bar","items":["one","two"]}"#.as_bytes();
        let result = calculate_results(&mut file_content, 2, false);
        assert_eq!(result.len(), 0);
    }

    // -- naive

    #[test]
    fn calculate_results_naive_correctly_creates_the_sets() {
        let mut file_content = r#"{"type":"B","foo":"bar","items":["one","two"]}
{"type":"B","foo":"bar","items":["one","two"]}
{"type":"A","foo":"bar","items":["one","two"]}
{"type":"C","foo":"bar","items":["one","two"]}
"#
        .as_bytes();
        let result = calculate_results_naive(&mut file_content, false);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn calculate_results_naive_adds_empty_lines_as_errors_creating_the_sets() {
        let mut file_content = r#"{"type":"B","foo":"bar","items":["one","two"]}
{"type":"B","foo":"bar","items":["one","two"]}
{"type":"A","foo":"bar","items":["one","two"]}

{"type":"C","foo":"bar","items":["one","two"]}
"#
        .as_bytes();
        let result = calculate_results_naive(&mut file_content, false);
        assert_eq!(result.len(), 4);
        assert!(result.get(ERROR_TYPE).is_some())
    }

    #[test]
    fn calculate_results_naive_adds_bad_formatted_json_as_errors_creating_the_sets() {
        let mut file_content = r#"{"type":"B" "foo":"bar","items":["one","two"]}
{"type":"B","foo":"bar","items":["one","two"]}
{"type":"A","foo":"bar","items":["one","two"]}
{"type":"C","foo":"bar","items":["one","two"]}
"#
        .as_bytes();
        let result = calculate_results_naive(&mut file_content, false);
        assert_eq!(result.len(), 4);
        assert!(result.get(ERROR_TYPE).is_some())
    }

    #[test]
    fn calculate_results_naive_adds_json_with_no_type_as_errors_creating_the_sets() {
        let mut file_content = r#"{"type1":"B" "foo":"bar","items":["one","two"]}
{"type":"B","foo":"bar","items":["one","two"]}
{"type":"A","foo":"bar","items":["one","two"]}
{"type":"C","foo":"bar","items":["one","two"]}
"#
        .as_bytes();
        let result = calculate_results_naive(&mut file_content, false);
        assert_eq!(result.len(), 4);
        assert!(result.get(ERROR_TYPE).is_some())
    }

    #[test]
    fn calculate_results_naive_takes_into_account_spaces_when_counting_bytes() {
        let mut file_content = r#"  {  "type":"B", "foo":"bar","items":["one","two"]}  
"#
        .as_bytes();
        let num_bytes = file_content.len();
        let result = calculate_results_naive(&mut file_content, false);
        assert_eq!(result.len(), 1);
        assert!(result.get(ERROR_TYPE).is_none());
        assert_eq!(result.get("B").map(|r| r.bytes), Some(num_bytes));
    }

    #[test]
    fn calculate_results_naive_takes_into_account_spaces_when_counting_bytes_even_when_invalid_json(
    ) {
        let mut file_content = r#"  {  "type":"B" "foo":"bar","items":["one","two"]}  
"#
        .as_bytes();
        let num_bytes = file_content.len();
        let result = calculate_results_naive(&mut file_content, false);
        let error = result.get(ERROR_TYPE).map(|r| r.bytes);
        assert_eq!(result.len(), 1);
        assert!(error.is_some());
        assert_eq!(error, Some(num_bytes));
    }
}
