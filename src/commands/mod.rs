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
		CommErr::Error($a.to_string(), $b.to_string())
	};
	($a:expr) => {
		CommErr::Error($a.to_string(), String::new())
	};
	(,$b:expr) => {
		CommErr::Error(String::new(), $b.to_string())
	};
	() => {
		CommErr::UnknownError
	}
}

macro_rules! syntax_error {
	() => {
		CommErr::SyntaxError
	}
}

macro_rules! handle {
	($a:expr,$b:expr) => {
		match $a {
			Ok(v) => v,
			Err(e) => return Err(error!($b, format!("{e}")))
		}
	};
	($a:expr) => {
		match $a {
			Ok(v) => v,
			Err(e) => return Err(error!(, format!("{e}")))
		}
	};
}

#[allow(unused_macros)]
macro_rules! handle_opt {
	($a:expr,$b:expr) => {
		match $a {
			Some(v) => v,
			None => return Err(error!($b))
		}
	};
	($a:expr) => {
		match $a {
			Some(v) => v,
			None => return Err(error!())
		}
	};
}

macro_rules! handle_syntax_opt {
	($a:expr) => {
		match $a {
			Some(v) => v,
			None => return Err(syntax_error!())
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
