#[derive(Debug)]
pub struct GeneratorConfig {
    pub dictionary_file_path: String,
    pub num_threads: usize,
    pub solutions_dest_file_path: Option<String>
}

impl GeneratorConfig {

    pub fn build(args: &[String]) -> Result<GeneratorConfig, &'static str> {

        // first arg is executable name
        if args.len() < 2 || args.len() > 4 {

            return Err("Inccorect number of args, accepts 2-3 args: \ndictioary source file path\nsolution destination file path\noptional amount of threads to use");
        }

        let dictionary_file_path = args[1].clone();

        let solutions_dest_file_path = if args.len() > 2  && args[2] != "" {
            Some(args[2].clone())
        } else {
            None
        };

        let num_threads = if args.len() == 4 {
            match args[3].clone().parse::<usize>() {
                Ok(num) => num,
                Err(_) => return Err("Could not parse the number of the number of threads argument")
            }
        } else {
            1
        };

        Ok(GeneratorConfig { dictionary_file_path, num_threads, solutions_dest_file_path })
    }
}
