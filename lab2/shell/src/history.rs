use std::{
    fs::{File, OpenOptions},
    io::{self, BufRead, Write},
};

pub struct History {
    file_name: String,
    lines: Vec<String>,
}

impl History {
    pub fn new(file_name: String) -> Result<History, io::Error> {
        let file = match File::open(&file_name) {
            Ok(file) => file,
            Err(_) => {
                File::create(&file_name).unwrap();
                File::open(&file_name)?
            }
        };
        let lines = io::BufReader::new(file)
            .lines()
            .map(|line| line.unwrap())
            .filter(|line| !line.is_empty())
            .collect::<Vec<String>>();
        Ok(History { file_name, lines })
    }

    pub fn push(&mut self, command: &String) -> () {
        self.lines.push(command.trim().to_string());
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.file_name)
            .unwrap();
        file.write_all(command.as_bytes()).unwrap();
    }

    pub fn size(&self) -> usize {
        self.lines.len()
    }

    pub fn get(&self, num: usize) -> Option<&String> {
        if num - 1 < self.size() {
            Some(&self.lines[num - 1])
        } else {
            None
        }
    }

    pub fn rget(&self, num: usize) -> Option<&String> {
        if self.size() - 1 - num < self.size() {
            Some(&self.lines[self.size() - 1 - num])
        } else {
            None
        }
    }
}
