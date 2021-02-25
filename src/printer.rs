use crate::models::TypeLineResults;
use prettytable::{cell, row, Table};

pub fn print_table(pretty_print: bool, results: &TypeLineResults) {
    if pretty_print {
        print_pretty_table(results);
    } else {
        print_lean_table(results);
    }
}

fn print_pretty_table(results: &TypeLineResults) {
    let mut table = Table::new();
    table.add_row(row!["TYPE", "TOTAL COUNT", "TOTAL BYTES"]);
    for (key, counter) in results {
        table.add_row(row![
            &key,
            counter.count.to_string(),
            counter.bytes.to_string()
        ]);
    }
    table.printstd();
}

fn print_lean_table(results: &TypeLineResults) {
    let mut table = String::new();
    for (key, counter) in results {
        table.push_str("TYPE: ");
        table.push_str(&key);
        table.push_str(" | TOTAL COUNT: ");
        table.push_str(counter.count.to_string().as_str());
        table.push_str(" | TOTAL BYTES: ");
        table.push_str(counter.bytes.to_string().as_str());
        table.push_str("\n");
    }
    println!("{}", table);
}
