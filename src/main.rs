mod solution_generator;
mod generator_config;

use crate::solution_generator::SolutionGeneratorThreadPool;

use crate::generator_config::GeneratorConfig;
use std::collections::HashMap;
use std::{process, env};
use std::time::Instant;
use std::error::Error;
use std::io::{BufReader, BufRead, BufWriter, Write};
use std::fs::File;

static ALPHABET: [&str; 26] = [
    "a", "b", "c", "d", "e", 
    "f", "g", "h", "i", "j", 
    "k", "l", "m", "n", "o",
    "p", "q", "r", "s", "t", 
    "u", "v", "w", "x", "y", 
    "z",
];

#[derive(PartialEq)]
#[derive(Debug)]
enum DictionaryErrors {
    InCorrectWordSize(String),
    Empty
}

fn main() {

    let args: Vec<String> = vec!["exec name".to_string(),"resources/dictionaries/words_medium.csv".to_string(), "test.csv".to_string(), "4".to_string()];//env::args().collect();

    let config = GeneratorConfig::build(&args).unwrap_or_else(|err| {
        println!("Problem parsing arguments: {err}");
        process::exit(1);
    });

    println!("{:?}", config);

    let dictionary = match read_dictionary_from_file(&config.dictionary_file_path){

        Ok(dictionary) => Box::new(dictionary),
        Err(err) => {
            eprintln!("Problem reading dictionary: {err}");
            process::exit(1)
        }
    };

    let prefix_map = match generate_starts_that_have_words(&dictionary) {
        Ok(prefix_map) => Box::new(prefix_map),
        Err(e) => panic!("Could not generate starts from dictionary {:?}", e),
    };

    let pool: SolutionGeneratorThreadPool = match SolutionGeneratorThreadPool::new(config.num_threads, &dictionary, prefix_map) {
        Ok(pool) => pool,
        Err(err) =>{
            eprintln!("Problem starting thread pool: {err}");
            process::exit(1)
        }
    };

    let now = Instant::now();

    let mut solutions: Vec<Vec<String>> = Vec::with_capacity(dictionary.len());

    for solution in pool.solution_receiver.iter() {
        //println!("received solution");
        solutions.push(solution);
    }

    println!("{:#?}", now.elapsed());

    if let Some(filename) = config.solutions_dest_file_path {
        
        save_solution_to_file(&filename, solutions);

    } else {

        println!("{:?}",solutions);
    }
}

/*
    From the file of the file_path read in a csv file that contains a list of words for a dictionary
    No Csv Headers 
    Sorts the dictionary
*/
fn read_dictionary_from_file(file_path: &str) -> Result<Vec<String>, Box<dyn Error>> {

    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut dictionary: Vec<String> = vec![];

    for line in reader.lines() {
        let mut words: Vec<String> = match line {
            Ok(line) => line.split(",").map(str::to_lowercase).filter(|s| s != "").collect(),
            Err(_) => panic!("could not read line")
        };

        dictionary.append(&mut words);
    }

    dictionary.sort();

    Ok(dictionary)
}

/* Generates a hashmap vectors containing words that correspond to a greatest common denomiator of substring.
  Will do this for all positions in a string besides the first as isnt needed */
fn generate_starts_that_have_words(dictionary: &Vec<String>) -> Result<HashMap<String, Vec<String>>, DictionaryErrors> {

    let word_size = match dictionary.get(0) {
        Some(word) => word.chars().count(),
        None => return Err(DictionaryErrors::Empty)
    };

    // dont need this hashmap this is just used to "seed" the actual hashmap
    let mut starts_word_map: HashMap<String, Vec<String>> = HashMap::new();

    for letter in ALPHABET {
        for word in dictionary {

            if word.len() != word_size {
                return Err(DictionaryErrors::InCorrectWordSize(format!("word [{}] has incorrect size needed {} found {}", word, word_size, word.len())))
            }

            if word.starts_with(letter) {
                let words_that_start_with = starts_word_map.entry(letter.to_owned()).or_insert_with(|| Vec::<String>::new());
                words_that_start_with.push(word.to_string());
            }
        }
    }

    for i in 1..word_size {

        let mut new_starts_map :HashMap<String, Vec<String>> = HashMap::new();

        // make new key next letter down in words
        for letter in ALPHABET {

            for prev_start in starts_word_map.keys() {

                //only care about the previously added keys
                if prev_start.len() != i {
                    continue;
                }

                let new_start = format!("{}{}", prev_start, letter);

                let words = starts_word_map.get(prev_start).expect("should always get a value");

                for word in words {

                    if !word.starts_with(&new_start) {
                        continue;
                    }

                    let new_start_list = new_starts_map.entry(new_start.clone()).or_insert_with(|| Vec::<String>::new());
                    new_start_list.push(word.to_string());
                }
            }
        }

        starts_word_map.extend(new_starts_map);
    }

    for letter in ALPHABET {
        starts_word_map.remove(letter);
    }

    Ok(starts_word_map)
}

fn save_solution_to_file(file_path: &String, solutions: Vec<Vec<String>>) {

    let file = File::create(file_path).unwrap();
    let mut file = BufWriter::new(file);
    for solution in solutions {
        let merged: String = solution.join(",");
        writeln!(file, "{}", merged).unwrap();
    }
}

#[test]
fn generate_starts() {

    let dictionary = vec!["based".to_string(), "bases".to_string(), "bassy".to_string(), "baton".to_string(), "belly".to_string(), "elses".to_string() ];

    let starts_generated = generate_starts_that_have_words(&dictionary).unwrap();

    let starts_excpected = HashMap::from([
        ("ba".to_string(), vec!["based".to_string(), "bases".to_string(), "bassy".to_string(), "baton".to_string()]),
        ("bas".to_string(), vec!["based".to_string(), "bases".to_string(), "bassy".to_string()]),
        ("base".to_string(), vec!["based".to_string(), "bases".to_string()]),
        ("based".to_string(), vec!["based".to_string()]),
        ("bases".to_string(), vec!["bases".to_string()]),
        ("bass".to_string(), vec!["bassy".to_string()]),
        ("bassy".to_string(), vec!["bassy".to_string()]),
        ("bat".to_string(), vec!["baton".to_string()]),
        ("bato".to_string(), vec!["baton".to_string()]),
        ("baton".to_string(), vec!["baton".to_string()]),
        ("be".to_string(), vec!["belly".to_string()]),
        ("bel".to_string(), vec!["belly".to_string()]),
        ("bell".to_string(), vec!["belly".to_string()]),
        ("belly".to_string(), vec!["belly".to_string()]),
        ("el".to_string(), vec!["elses".to_string()]),
        ("els".to_string(), vec!["elses".to_string()]),
        ("else".to_string(), vec!["elses".to_string()]),
        ("elses".to_string(), vec!["elses".to_string()]),
        ]);

    for key in starts_excpected.keys() {

        assert!(starts_excpected.contains_key(key));
        assert!(starts_generated.contains_key(key));

        let vec1 = starts_excpected.get(key).unwrap();
        let vec2 = starts_generated.get(key).unwrap();

        assert_eq!(vec1.len(), vec2.len());

        assert_eq!(vec1, vec2);
    }

}

#[test]
fn generate_starts_empty_dictionary() {

    let dictionary: Vec<String> = Vec::new();

    let starts = generate_starts_that_have_words(&dictionary);

    assert_eq!(DictionaryErrors::Empty, starts.unwrap_err());
}

#[test]
fn generate_starts_has_incorrect_sized_word() {

    let s1:String = "abcdefg".to_string();
    let s2:String = "hijklmno".to_string();

    let dictionary = vec![s1.clone(), s2.clone()];

    let starts = generate_starts_that_have_words(&dictionary);

    assert_eq!(DictionaryErrors::InCorrectWordSize( format!("word [{}] has incorrect size needed {} found {}", s2, s1.len(), s2.len())), starts.unwrap_err());
}

#[test]
fn write_one_solution() {

    let solutions: Vec<Vec<String>> = vec![vec!["word1".to_string(),"word2".to_string(),"word3".to_string(),"word4".to_string(),"word5".to_string()]];
    let solution_copy = solutions.clone();

    let file_path = "test.csv".to_string();
    save_solution_to_file(&file_path, solutions);

    let file = File::open(&file_path).unwrap();
    let reader = BufReader::new(file);
    let mut solutions_read: Vec<Vec<String>> = vec![];

    for line in reader.lines() {
        let words: Vec<String> = match line {
            Ok(line) => line.split(",").map(str::to_lowercase).filter(|s| s != "").collect(),
            Err(_) => panic!("could not read line")
        };

        solutions_read.push(words);
    }
    
    assert_eq!(solution_copy, solutions_read);
}