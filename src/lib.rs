//
////////////////////////
// Handles os-based i/o
////////////////////////
pub mod files {
    use crate::grouper;
    use crate::models::{Student, StudentBuilder};
    use csv::StringRecord;
    use serde::Deserialize;
    use std::io::Read;
    use std::net::TcpStream;
    use std::path::Path;
    use std::{
        env,
        fs::{create_dir, read_dir, remove_file, File},
        io::Write,
    };

    #[derive(Deserialize, Clone)]
    pub struct FileHandler {
        pub temp_path: String,
    }

    impl Default for FileHandler {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FileHandler {
        pub fn new() -> FileHandler {
            FileHandler {
                temp_path: format!(
                    "{}grouper-data",
                    env::temp_dir().to_string_lossy().into_owned()
                ),
            }
        }

        //

        pub fn init_base_dir(&self) -> std::io::Result<()> {
            let path = self.get_temp_path();
            create_dir(path)?;
            Ok(())
        }

        //

        pub fn read_and_return_json(
            &self,
            filename: &str,
        ) -> Result<String, Box<dyn std::error::Error>> {
            let full_path = self.get_full_path(filename);

            let file = File::open(full_path).expect("Failed to read json file");

            let file_contents = Self::file_to_string(file);
            Ok(file_contents)
        }

        //

        pub fn read_and_return_students(&self, filename: &str) -> Result<Vec<Student>, ()> {
            let full_path = self.get_full_path(filename);
            let file = File::open(full_path).expect("Failed to read json file");

            let file_contents = Self::file_to_string(file);

            if let Ok(result) = grouper::Utils::students_from_json(&file_contents) {
                Ok(result)
            } else {
                Err(())
            }
        }

        //

        fn file_to_string(mut file: File) -> String {
            let mut file_contents = String::new();

            file.read_to_string(&mut file_contents)
                .expect("Failed to read json file.");
            file_contents
        }

        //

        fn get_full_path(&self, filename: &str) -> String {
            let temp_path = self.get_temp_path();
            format!("{}\\{}", &temp_path, &filename)
        }

        //

        pub fn write_json(
            &self,
            data: Vec<Student>,
            filename: &str,
        ) -> Result<String, Box<dyn std::error::Error>> {
            if let false = self.check_for_dir() {
                println!("Creating base directory for Grouper Desktop user.");
                self.init_base_dir().unwrap();
            } else {
                println!("Base directory found. Proceeding to write file.");
            }
            let write_path = format!("{}\\{}.json", self.get_temp_path(), filename);
            let mut file = File::create(&write_path)?;

            let json = serde_json::to_string(&data)?;
            file.write_all(json.as_bytes())?;

            Ok(format!(
                "SUCCESS writing JSON to temp directory ::: @ ::: {}",
                write_path
            ))
        }

        //

        pub fn delete_file(&self, filename: &str) -> Result<String, Box<dyn std::error::Error>> {
            let path = self.get_temp_path();
            let full_path = format!("{}\\{}", &path, &filename);
            match remove_file(full_path) {
                Ok(_) => Ok("File deleted successfully".to_string()),
                Err(e) => Err(format!("Error deleting file: {}", e).into()),
            }
        }

        //

        fn get_temp_path(&self) -> String {
            self.temp_path.clone()
        }

        //

        fn check_for_dir(&self) -> bool {
            let temp = &self.get_temp_path();
            let path = Path::new(temp);
            path.is_dir()
        }

        //

        pub fn read_directory(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
            let path = self.get_temp_path();
            let dir = read_dir(path).unwrap();

            let mut file_list: Vec<String> = Vec::new();

            for file in dir {
                let file_name = file.unwrap().file_name();
                println!("{:?}", file_name);
                file_list.push(file_name.into_string().expect("Failed to parse file name."));
            }

            Ok(file_list)
        }
        //
        pub fn student_from_record(idx: usize, row: Result<StringRecord, csv::Error>) -> Student {
            let r = row.expect("Unable to parse string record");

            fn parse_avg(r: StringRecord) -> f32 {
                r.get(42).unwrap().parse::<f32>().unwrap_or(0.0_f32)
            }

            StudentBuilder::new()
                .id(idx as u32)
                .name(r.get(0).unwrap().to_string())
                .email(r.get(2).unwrap().to_string())
                .avg(parse_avg(r))
                .group(0)
                .build()
        }
        //

        pub fn network_available() -> bool {
            TcpStream::connect("8.8.8.8:53").is_ok()
        }

        //

        pub fn temp_data_available(&self) -> bool {
            let path = format!("{}{}", self.get_temp_path(), "\\grouper-students.json");
            let file_path = Path::new(&path);
            println!("{}", &path);
            if let true = file_path.exists() {
                println!("path found");
                return true;
            }
            false
        }
    }
}

//////////////////
/// Handles student group manipulations
//////////////////
pub mod grouper {

    use crate::err_handle::{self};
    use crate::models::Student;
    use rand::Rng;
    use std::collections::{BTreeMap, HashMap};

    // Main Handler - Balancer

    type Students = Vec<Student>;

    //
    // Collection Transformation State
    //
    #[derive(Debug, Clone)]
    pub struct GroupsMap(BTreeMap<u16, Vec<Student>>);

    impl GroupsMap {
        pub fn new(num_students: u16, group_size: u16) -> Self {
            let tree_map = BTreeMap::new();
            let mut new_map = GroupsMap(tree_map);
            new_map.populate(num_students, group_size);
            new_map
        }

        fn populate(&mut self, num_students: u16, group_size: u16) -> &mut Self {
            let num_groups: u16 = (num_students as f32 / group_size as f32).floor() as u16;

            for num in 1..=num_groups {
                self.0.insert(num, vec![]);
            }
            self
        }
    }

    //
    // Utilities / Auxiliaries
    //
    #[derive(Debug, Clone, Copy)]
    pub struct Utils;

    impl Utils {
        //

        fn rand_idx(vec_length: &usize) -> usize {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..*vec_length)
        }

        //

        pub fn num_groups(num_students: u16, group_size: u16) -> u16 {
            let res: f32 = num_students as f32 / group_size as f32;
            res.floor() as u16
        }

        //

        fn mean(floats: &[f32]) -> f32 {
            floats.iter().fold(0 as f32, |acc, n| acc + n) / floats.len() as f32
        }

        //

        fn diffs(floats: &[f32], mean: &f32) -> Vec<f32> {
            floats.iter().fold(vec![], |mut acc: Vec<f32>, &val| {
                acc.push((val - mean).abs());
                acc
            })
        }

        //

        fn square_all(floats: &[f32]) -> Vec<f32> {
            floats.iter().map(|float| float.powi(2)).collect()
        }

        //

        fn round_to_dec_count(value: f32, dec_count: i32) -> f32 {
            let multi = 10.0_f32.powi(dec_count);
            (value * multi).round() / multi
        }

        //

        pub fn std_dev(floats: Vec<f32>) -> f32 {
            // 1. Calculate the mean of the vector.
            let mean = Self::mean(&floats);
            // 2. Calculate the difference between each element of the vector and the mean.
            let differences = Self::diffs(&floats, &mean);
            // 3. Square the differences.
            let all_squared: Vec<f32> = Self::square_all(&differences);
            // 4. Calculate the mean of the squared differences.
            let mean_of_squared: f32 = Self::mean(&all_squared);
            // 5. Take the square root of the mean of the squared differences to get the standard deviation.
            let sd: f32 = mean_of_squared.sqrt();

            sd
        }

        //

        pub fn sort_students(vec_of_students: &Students) -> Students {
            let mut students = vec_of_students.clone();
            students.sort_by(|a, b| a.avg.partial_cmp(&b.avg).unwrap());
            students
        }

        //

        pub fn group_avgs_map(groups: &GroupsMap) -> HashMap<u16, f32> {
            let mut map = HashMap::new();

            for (k, v) in groups.0.clone().into_iter() {
                let group_avg = v.iter().fold(0 as f32, |mut acc, val| {
                    acc += val.avg;
                    acc
                }) / v.len() as f32;
                map.entry(k).or_insert(group_avg);
            }

            map
        }

        //

        pub fn group_avgs_vec(map: HashMap<u16, f32>) -> Vec<f32> {
            let mut avgs = vec![];

            for (_, v) in map.into_iter() {
                avgs.push(Self::round_to_dec_count(v, 2));
            }

            avgs
        }

        //

        pub fn send_group_avgs(groups_json: String) -> Result<String, err_handle::Error> {
            let data: Students =
                serde_json::from_str(&groups_json).expect("Failed to parse vector from json ...");
            println!("{:?}", data);

            Ok("".into())
        }

        //

        pub fn students_from_json(json_str: &str) -> Result<Vec<Student>, err_handle::Error> {
            let people: Vec<Student> = serde_json::from_str(json_str)
                .expect("Failed to parse students from json string ... ");
            Ok(people)
        }

        //

        pub fn treemap_to_json(
            groups: BTreeMap<u16, Vec<Student>>,
        ) -> Result<String, Box<dyn std::error::Error>> {
            let json = serde_json::to_string(&groups)?;
            Ok(json)
        }

        //

        pub fn groups_from_json(
            json_str: &str,
        ) -> Result<BTreeMap<u16, Vec<Student>>, err_handle::Error> {
            let groups: BTreeMap<u16, Vec<Student>> = serde_json::from_str(json_str)
                .expect("Failed to parse groups from json string ... ");
            Ok(groups)
        }

        //

        fn get_random_student(students: &mut Students) -> (&mut Student, usize) {
            let rand_idx = Self::rand_idx(&students.len());
            let rand_student = &mut students[rand_idx];
            (rand_student, rand_idx)
        }

        pub fn random_assignment(
            current: u16,
            mut students: Students,
            mut groups_map: GroupsMap,
            num_groups: u16,
        ) -> GroupsMap {
            if students.len() == 0 {
                return groups_map;
            };

            let (random_student, rand_idx) = Self::get_random_student(&mut students);

            let mut current_group = current;
            random_student.set_group(current_group);

            let mut new_vec = groups_map.0.get(&current_group).unwrap().clone();
            new_vec.push(random_student.to_owned());

            groups_map.0.insert(current_group, new_vec);

            if current_group == num_groups {
                current_group = 1;
            } else {
                current_group += 1;
            }

            students.remove(rand_idx);

            Self::random_assignment(current_group, students, groups_map, num_groups)

            //
        }

        //

        fn balance(
            students: Vec<Student>,
            group_size: u16,
            _target_sd: u8,
        ) -> BTreeMap<u16, Vec<Student>> {
            //
            let groups_map = GroupsMap::new(students.len() as u16, group_size);
            let sorted = Self::sort_students(&students);
            let num_groups = Self::num_groups(sorted.len() as u16, group_size);
            //

            Self::random_assignment(1, sorted, groups_map, num_groups).0

            //
        }

        //

        pub fn multi_balance(
            _num_workers: u8,
            students: Vec<Student>,
            group_size: u16,
            target_sd: u8,
        ) -> BTreeMap<u16, Vec<Student>> {
            //
            // TODO
            // Implement multi-threading to batch job too large for single stack.
            // - Assign the same work to all threads.
            // - Use whichever result returns first and dispense with operations on busy threads?
            // - Probably a bad idea ....

            Self::balance(students, group_size, target_sd)

            //
        }

        //
    }

    #[cfg(test)]
    mod tests {
        use super::Utils;

        #[test]
        fn test_standard_deviation() {
            let test_vector = vec![
                // Each group's average as f32
                79.08, 83.15, 96.23, 85.11, 90.73, 77.79, 80.34,
            ];
            // 1. Calculate the mean of the vector.
            let mean = Utils::mean(&test_vector);
            // 2. Calculate the difference between each element of the vector and the mean.
            let differences = Utils::diffs(&test_vector, &mean);
            // 3. Square the differences.
            let all_squared: Vec<f32> = Utils::square_all(&differences);
            // 4. Calculate the mean of the squared differences.
            let mean_of_squared: f32 = Utils::mean(&all_squared);
            // 5. Take the square root of the mean of the squared differences to get the standard deviation.
            let sd: f32 = mean_of_squared.sqrt();

            assert_eq!(sd, 6.2126956 as f32);
        }
    }
}

//////////////////
/// Builders
//////////////////
pub mod models {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
    pub struct Student {
        id: u32,
        name: String,
        pub avg: f32,
        group: u16,
        email: String,
    }
    impl Student {
        pub fn set_group(&mut self, g: u16) {
            self.group = g;
        }
    }

    pub struct StudentBuilder {
        id: Option<u32>,
        name: Option<String>,
        avg: Option<f32>,
        group: Option<u16>,
        email: Option<String>,
    }

    impl Default for StudentBuilder {
        fn default() -> Self {
            Self::new()
        }
    }

    impl StudentBuilder {
        pub fn new() -> Self {
            StudentBuilder {
                id: None,
                name: None,
                avg: None,
                group: None,
                email: None,
            }
        }

        pub fn id(mut self, id: u32) -> Self {
            self.id = Some(id);
            self
        }

        pub fn name(mut self, name: String) -> Self {
            self.name = Some(name);
            self
        }

        pub fn avg(mut self, avg: f32) -> Self {
            self.avg = Some(avg);
            self
        }

        pub fn group(mut self, group: u16) -> Self {
            self.group = Some(group);
            self
        }

        pub fn email(mut self, email: String) -> Self {
            self.email = Some(email);
            self
        }

        pub fn build(self) -> Student {
            Student {
                id: self.id.unwrap(),
                name: self.name.unwrap(),
                avg: self.avg.unwrap(),
                group: self.group.unwrap(),
                email: self.email.unwrap(),
            }
        }
    }
}

//////////////////
/// Multithreading
//////////////////
pub mod mutant {}

//////////////////
/// Error Handling
//////////////////
pub mod err_handle {
    use std::fmt;

    #[derive(Debug, thiserror::Error)]

    pub enum Error {
        #[error(transparent)]
        Io(#[from] std::io::Error),
        Std(#[from] Box<dyn std::error::Error>),
    }

    impl serde::Serialize for Error {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::ser::Serializer,
        {
            serializer.serialize_str(self.to_string().as_ref())
        }
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Error reading file ... ")
        }
    }
}

pub use err_handle::Error;
pub use files::FileHandler;
pub use grouper::{GroupsMap, Utils};
pub use models::{Student, StudentBuilder};
