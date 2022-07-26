use std::fs;
use std::io;
use std::io::{Read, Write, Seek, BufRead};
use std::path::Path;
use serde_json::Value;

pub struct Database {
	db_tmp_file: fs::File,
	data: Value
}

impl Database {
	pub fn open(path_str: &str) -> Result<Database, String> {
		let path = Path::new(path_str);
		let db_file_path = path.join("database.json");
		let db_tmp_file_path = path.join("database_tmp.json");
		if !path.is_dir() {
			if let Err(e) = fs::create_dir(path_str) {
				return Err(format!("Unable to create the database's directory: {e}"));
			}
		}
		if !db_file_path.is_file() {
			match fs::File::create(db_file_path.to_str().unwrap()) {
				Ok(mut file) => writeln!(file, "{{}}").unwrap(),
				Err(e) => return Err(format!("Unable to create the database's database.json file: {e}"))
			};
		}
		if !db_tmp_file_path.is_file() {
			if let Err(e) = fs::File::create(db_tmp_file_path.to_str().unwrap()) {
				return Err(format!("Unable to create the database's database_tmp.json file: {e}"));
			}
		}
		let db_tmp_file = match fs::File::options().read(true).write(true).append(true).open(db_tmp_file_path.to_str().unwrap()) {
			Ok(v) => v,
			Err(e) => return Err(format!("Unable to open the database's database_tmp.json file: {e}"))
		};
		let mut db_file = match fs::File::options().read(true).write(true).open(db_file_path.to_str().unwrap()) {
			Ok(v) => v,
			Err(e) => return Err(format!("Unable to open the database's database.json file: {e}"))
		};
		let mut db_file_content = String::new();
		db_file.read_to_string(&mut db_file_content).unwrap();
		let data: Value = match serde_json::from_str(&*db_file_content) {
			Ok(v) => v,
			Err(e) => return Err(format!("Unable to parse the database's database.json file as JSON: {e}"))
		};
		if !data.is_object() {
			return Err("The database's database.json file must contain a JSON object in its root".to_string())
		}
		let mut db = Database {
			db_tmp_file: db_tmp_file.try_clone().unwrap(),
			data: data
		};
		if db_tmp_file.metadata().unwrap().len() != 0 {
			println!("Applying changes from database_tmp.json to database.json");
			for line in io::BufReader::new(db_tmp_file.try_clone().unwrap()).lines() {
				let new_data: Value = serde_json::from_str(&*line.unwrap()).unwrap();
				let path = match &new_data[0] {
					Value::Array(v) => {
						v.iter().map(|x| match x {
							Value::String(v) => v.as_str(),
							_ => ""
						}).collect()
					},
					_ => Vec::new()
				};
				db.silently_set(path.as_slice(), new_data[1].clone()).unwrap();
			}
			db_tmp_file.set_len(0).unwrap();
			db_file.set_len(0).unwrap();
			db_file.rewind().unwrap();
			writeln!(db_file, "{}", db.data.to_string()).unwrap();
			println!("Done");
		}
		Ok (
			db
		)
	}
	fn silently_set(&mut self, path: &[&str], value: Value) -> Result<(), ()> {
		let mut data = &mut self.data;
		for i in 0..(path.len() - 1) {
			let key = path[i];
			if data.get(key).is_some() {
				data = match data.is_object() {
					true => data.get_mut(key).unwrap(),
					false => return Err(())
				};
			}
			else {
				data[key] = serde_json::json!({});
				data = match data.is_object() {
					true => data.get_mut(key).unwrap(),
					false => return Err(())
				};
			}
		}
		if value.is_null() {
			data.as_object_mut().unwrap().remove(path[path.len() - 1]);
		}
		else {
			data[path[path.len() - 1]] = value;
		}
		Ok(())
	}
	pub fn set(&mut self, path: &[&str], value: Value) -> Result<(), ()> {
		self.silently_set(path, value.clone())?;
		writeln!(&mut self.db_tmp_file, "{}", serde_json::to_string(&serde_json::json!([path.to_vec(), value])).unwrap()).unwrap();
		Ok(())
	}
	pub fn get(&mut self, path: &[&str]) -> &Value {
		let mut data = &self.data;
		for key in path {
			if data.get(key).is_some() {
				data = match data.is_object() {
					true => match data.get(key) {
						Some(v) => v,
						None => &Value::Null
					},
					false => return &Value::Null
				};
			}
			else {
				return &Value::Null;
			}
		}
		data
	}
}
