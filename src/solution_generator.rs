use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;
use std::{
    sync::{mpsc, Arc},
    thread,
};
use tqdm::tqdm;


enum ExecuteMessage {
    Word(String),
    Terminate,
}

pub enum ThreadPoolError {
    ZeroSizedPool,
    ZeroSizedDictionary,
    ZeroSizedPrefixMap
}

impl fmt::Display for ThreadPoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ThreadPoolError::ZeroSizedPool => "Number of threads is less than or equal to 0",
            ThreadPoolError::ZeroSizedDictionary => "Dictionary is empty",
            ThreadPoolError::ZeroSizedPrefixMap => "Prefix map is empty",
        }
        .fmt(f)
    }
}

pub struct SolutionGeneratorThreadPool {
    pub solution_receiver: Receiver<Vec<String>>,
}

impl SolutionGeneratorThreadPool {

    pub fn new(num_threads: usize, dictionary: &Box<Vec<String>>, prefix_map: Box<HashMap<String, Vec<String>>>) -> Result<SolutionGeneratorThreadPool, ThreadPoolError> {

        if num_threads <= 0 {
            return Err(ThreadPoolError::ZeroSizedPool)
        }

        if dictionary.len() == 0 {
            return Err(ThreadPoolError::ZeroSizedDictionary)
        }

        if prefix_map.len() <= 0 {
            return Err(ThreadPoolError::ZeroSizedPrefixMap)
        }

        let word_size = dictionary.get(0).unwrap().chars().count();

        let prefix_map_arc = Arc::new(prefix_map);

        let mut workers: Vec<SolutionGeneratorWorker> = Vec::with_capacity(num_threads);

        let (word_sender, word_receiver) = mpsc::sync_channel::<ExecuteMessage>(8);
        let word_receiver = Arc::new(Mutex::new(word_receiver));

        let (solution_sender, solution_receiver) = mpsc::channel::<Vec<String>>();

        for _ in 0..num_threads {

            workers.push(SolutionGeneratorWorker::new(
                dictionary.clone(),
                Arc::clone(&prefix_map_arc),
                solution_sender.clone(),
                word_receiver.clone(),
                word_size
            ));
        }

        let dict_clone = dictionary.clone();

        thread::spawn( move || {

            for word in tqdm(dict_clone.iter()) {
                word_sender.send(ExecuteMessage::Word(word.to_string())).unwrap();
            }

            for _ in 0..num_threads {
                word_sender.send(ExecuteMessage::Terminate).unwrap();
            }
        });

        Ok(SolutionGeneratorThreadPool {solution_receiver })
    }
}

struct SolutionGeneratorWorker {
}

impl SolutionGeneratorWorker {

    fn new(dictionary: Box<Vec<String>>, prefix_map_arc: Arc<Box<HashMap<String, Vec<String>>>>, 
        solution_sender: Sender<Vec<String>>, word_receiver: Arc<Mutex<Receiver<ExecuteMessage>>>,
        word_size: usize) -> SolutionGeneratorWorker {

        let mut solution_generator = SolutionGenerator::new(
            dictionary,
            prefix_map_arc,
            solution_sender,
            word_size
        );

        thread::spawn(move || 

            loop {

                let execute_message = word_receiver.lock().unwrap().recv().unwrap();

                match execute_message {
                    ExecuteMessage::Word(word) => solution_generator.run(word),
                    ExecuteMessage::Terminate => {
                        break;
                    }
                };
            }
        );

        SolutionGeneratorWorker {}
    }

}

struct SolutionGenerator {
    dictionary: Box<Vec<String>>,
    last_row_index: usize,
    prefix_map_arc: Arc<Box<HashMap<String, Vec<String>>>>,
    solution_sender: Sender<Vec<String>>,
    word_size: usize
}

impl SolutionGenerator {

    fn new(dictionary: Box<Vec<String>>, prefix_map_arc: Arc<Box<HashMap<String, Vec<String>>>>,
        solution_sender: Sender<Vec<String>>, word_size: usize) -> SolutionGenerator {

        SolutionGenerator {
            dictionary,
            last_row_index: word_size -1,
            prefix_map_arc,
            solution_sender,
            word_size
        }
    }

    fn run(&mut self, word: String) {

        let mut initial_puzzle: Vec<String> = Vec::with_capacity(self.word_size);
        initial_puzzle.push(word);

        self.find_solutions(&mut initial_puzzle, 1);
    }
    
    fn find_solutions(&self, puzzle: &mut Vec<String>, row_index: usize) {
    
        let potential_columns = construct_potential_transposed_puzzle(puzzle);
    
        let mut bad_starts = vec![ "".to_string(); self.last_row_index];
    
        for word in self.dictionary.iter() {
    
            if skip_word(&word, &bad_starts, puzzle) {
                continue;
            }
    
            let (fit, last_column_index_checked) = 
                if row_index == self.last_row_index
                    {self.last_word_fits(puzzle, &word, &potential_columns)} 
                else 
                    {self.word_fits(&word, &potential_columns)};
    
            if fit {

                // solution found
                if last_column_index_checked ==  self.last_row_index {

                    let mut temp_puzzle_solution = puzzle.clone();
                    temp_puzzle_solution.push(word.clone());

                    //println!("solution {:?}", temp_puzzle_solution);
                    self.solution_sender.send(temp_puzzle_solution).expect("Sender should always be able to send");
                }

                puzzle.push(word.clone());

                self.find_solutions(puzzle, row_index + 1);

                puzzle.pop();
    
            } else {

                // if it failed on the last column index no need to record it as words are unique
                if last_column_index_checked == self.last_row_index {
                    continue;
                }

                let bad_start: String = word.chars().take(last_column_index_checked+1).collect();

                bad_starts[last_column_index_checked] = bad_start;
            }
        }
    
    }
    
    fn word_fits(&self, word: &String, potential_columns: &Vec<String>) -> (bool, usize) {
    
        let size = word.len();
        if potential_columns.len() != size {
            panic!("Potential coulmns [{}] and word [{}] have different lengths", potential_columns.len(), size);
        }
    
        for i in 0..size {
    
            let start_string = potential_columns.get(i).unwrap();
    
            let new_potential_column = format!("{}{}", start_string, word.chars().nth(i).unwrap());
    
            if !&self.prefix_map_arc.contains_key(&new_potential_column) {
                return (false, i)
            }
        }
    
        (true, size)
    }
    
    fn last_word_fits(&self, puzzle: &Vec<String>, word: &String, potential_columns: &Vec<String>) -> (bool, usize) {
    
        for i in 0..word.len() {
    
            let start_string = potential_columns.get(i).unwrap();
    
            let new_potential_column = format!("{}{}", start_string, word.chars().nth(i).unwrap());
    
            if i == 0 && would_be_transposed_row(&puzzle.get(0).unwrap(), &new_potential_column) {
                return (false, i);
            }
    
            if self.does_column_fit(&start_string, &new_potential_column, puzzle) {

                return (false, i)
            }
        }
    
        (true, word.len() -1)
    }

    fn does_column_fit(&self, partial_column: &String, column: &String, puzzle: &Vec<String>) -> bool {
        !self.prefix_map_arc.contains_key(partial_column)
        || !self.prefix_map_arc.get(partial_column).unwrap().contains(column) 
        || puzzle.contains(column)
    }
    
}

/* if the column is alphabetically before the row than the column has already
 been tried at that row previously, so this would result in a transposed solution */
fn would_be_transposed_row(row: &String, column: &String) -> bool {
    column.cmp(row) == Ordering::Less
}

#[test]
fn is_transposed_row(){

    let row = "bases".to_string();
    let column = "based".to_string();

    assert!(would_be_transposed_row(&row, &column))
}

#[test]
fn isnt_transposed_row(){
    
    let row = "based".to_string();
    let column = "bases".to_string();

    assert!(!would_be_transposed_row(&row, &column))
}


/* do not process this word as it starts with chars that have been identified as a dead end or are in the puzzle */
fn skip_word(word: &String, bad_starts: &Vec<String>, puzzle: &Vec<String>) -> bool {
    bad_starts.iter().filter(|bad_start| !bad_start.is_empty()).any(|bad_start| word.starts_with(bad_start)) || puzzle.contains(word)
}

/* transposes the rows of a puzzle into columns to be used in determining if the columns of a puzzle will be valid */
fn construct_potential_transposed_puzzle(puzzle: &Vec<String>) -> Vec<String> {
    
    let mut potential_transposed_puzzle = vec!["".to_string() ; puzzle.get(0).expect("puzzle to have the first row populated").len()];

    for word in puzzle {
        for (i, ch) in word.chars().enumerate() {

            if let Some(potential_column)  = potential_transposed_puzzle.get_mut(i)  {
                potential_column.push(ch);
            };
        }
    }

    potential_transposed_puzzle
}

#[test]
fn trasnspose() {
    let puzzle =  vec!["budge".to_string() ,"enter".to_string(),"alien".to_string(),"scant".to_string(),"eerie".to_string()];

    let potential_columns = construct_potential_transposed_puzzle(&puzzle);

    assert_eq!(vec!["bease".to_string() ,"unlce".to_string(),"dtiar".to_string(),"geeni".to_string(),"ernte".to_string()], potential_columns);
}