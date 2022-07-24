use crate::core::*;
use super::CHLOE;
use serde_json::Value;
use std::string::String;
use std::time::Duration;
use futures::StreamExt;
use serenity::builder::{CreateComponents, CreateActionRow, CreateButton};
use serenity::model::application::interaction::InteractionResponseType;
use serenity::builder::CreateApplicationCommandOption;
use serenity::model::application::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::CommandDataOptionValue;
use serenity::model::application::component::ButtonStyle;

macro_rules! func {
	($a:block) => {
		Box::pin(async move {
			$a
		})
	};
}

macro_rules! error {
	($a:expr,$b:expr) => {
		Err(CommErr::Error($a.to_string(), $b.to_string()))
	};
	($a:expr) => {
		Err(CommErr::Error($a.to_string(), String::new()))
	};
	(,$b:expr) => {
		Err(CommErr::Error(String::new(), $b.to_string()))
	};
	() => {
		Err(CommErr::UnknownError)
	}
}

macro_rules! syntax_error {
	() => {
		Err(CommErr::SyntaxError)
	}
}

macro_rules! handle {
	($a:expr,$b:expr) => {
		match $a {
			Ok(v) => v,
			Err(e) => return error!($b, format!("{e}"))
		}
	};
	($a:expr) => {
		match $a {
			Ok(v) => v,
			Err(e) => return error!(, format!("{e}"))
		}
	};
}

macro_rules! handle_opt {
	($a:expr,$b:expr) => {
		match $a {
			Some(v) => v,
			None => return error!($b)
		}
	};
	($a:expr) => {
		match $a {
			Some(v) => v,
			None => return error!()
		}
	};
}

macro_rules! handle_syntax_opt {
	($a:expr) => {
		match $a {
			Some(v) => v,
			None => return syntax_error!()
		}
	};
}

macro_rules! svec {
	($($x:expr),+ $(,)?) => {
		vec![$($x.to_string()),+]
	};
}

mod misc;

pub fn commands<'a>() -> Vec<Command<'a>> {
	let mut commands: Vec<Command> = Vec::new();
	commands.append(&mut misc::commands());
	for command in commands.iter_mut() {
		let mut args = Vec::new();
		if !command.options.is_empty() {
			for option_fn in command.options.iter() {
				let mut option = CreateApplicationCommandOption::default();
				option_fn(&mut option);
				let option_name = option.0.get("name").unwrap().as_str().unwrap();
				let option_string = match option.0.get("required").unwrap().as_bool().unwrap() {
					true => format!("<{}>", option_name),
					false => format!("[{}]", option_name)
				};
				args.push(option_string);
			}
		}
		command.args = args.join(" ");
	}
	commands
}