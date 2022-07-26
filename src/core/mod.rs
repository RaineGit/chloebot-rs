pub mod db;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::collections::HashMap;
use db::Database;
use std::sync::{Mutex, Arc};
use std::pin::Pin;
use std::future::Future;
use serde_json::Value;
use serenity::builder::{CreateMessage, CreateComponents, CreateApplicationCommandOption, CreateInteractionResponseFollowup};
use serenity::model::channel::Message;
use serenity::model::guild::Member;
use serenity::model::user::User;
use serenity::model::prelude::{Role, Attachment, PartialMember, PartialChannel};
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::application::interaction::application_command::{ApplicationCommandInteraction, CommandDataOption, CommandDataOptionValue};
use super::CHLOE;

pub struct CommOption {
	pub name: String,
	pub value: Option<Value>,
	pub kind: CommandOptionType,
	pub options: CommOptions,
	pub resolved: Option<CommandDataOptionValue>,
	pub focused: bool
}

impl CommOption {
	pub fn new(option: CommandDataOption) -> Self {
		Self {
			name: option.name,
			value: option.value,
			kind: option.kind,
			options: CommOptions::new(option.options),
			resolved: option.resolved,
			focused: option.focused
		}
	}
}

pub struct CommOptions(pub HashMap<String, CommOption>);

impl CommOptions {
	pub fn new(options: Vec<CommandDataOption>) -> Self {
		let mut new_options: HashMap<String, CommOption> = HashMap::new();
		for option in options.into_iter() {
			new_options.insert(option.name.clone(), CommOption::new(option));
		}
		Self(new_options)
	}
	pub fn get(&self, name: &str) -> Option<&CommandDataOptionValue> {
		match self.0.get(name) {
			Some(v) => match &v.resolved {
				Some(v) => Some(v),
				None => None
			},
			None => None
		}
	}
	pub fn get_string(&self, name: &str) -> Option<String> {
		match self.get(name) {
			Some(v) => match v {
				CommandDataOptionValue::String(v) => Some(v.clone()),
				_ => None
			},
			None => None
		}
	}
	pub fn get_int(&self, name: &str) -> Option<i64> {
		match self.get(name) {
			Some(v) => match v {
				CommandDataOptionValue::Integer(v) => Some(*v),
				_ => None
			},
			None => None
		}
	}
	pub fn get_number(&self, name: &str) -> Option<f64> {
		match self.get(name) {
			Some(v) => match v {
				CommandDataOptionValue::Number(v) => Some(*v),
				_ => None
			},
			None => None
		}
	}
	pub fn get_bool(&self, name: &str) -> Option<bool> {
		match self.get(name) {
			Some(v) => match v {
				CommandDataOptionValue::Boolean(v) => Some(*v),
				_ => None
			},
			None => None
		}
	}
	pub fn get_user(&self, name: &str) -> Option<(&User, Option<&PartialMember>)> {
		match self.get(name) {
			Some(v) => match v {
				CommandDataOptionValue::User(v0, v1) => Some((v0, v1.as_ref())),
				_ => None
			},
			None => None
		}
	}
	pub fn get_channel(&self, name: &str) -> Option<&PartialChannel> {
		match self.get(name) {
			Some(v) => match v {
				CommandDataOptionValue::Channel(v) => Some(v),
				_ => None
			},
			None => None
		}
	}
	pub fn get_role(&self, name: &str) -> Option<&Role> {
		match self.get(name) {
			Some(v) => match v {
				CommandDataOptionValue::Role(v) => Some(v),
				_ => None
			},
			None => None
		}
	}
	pub fn get_attachment(&self, name: &str) -> Option<&Attachment> {
		match self.get(name) {
			Some(v) => match v {
				CommandDataOptionValue::Attachment(v) => Some(v),
				_ => None
			},
			None => None
		}
	}
	pub fn get_options(&self, name: &str) -> Option<&CommOptions> {
		match self.0.get(name) {
			Some(v) => Some(&v.options),
			None => None
		}
	}
}

pub struct CommandParams {
	pub prefix: String,
	pub db: Arc<Mutex<Database>>,
	pub ctx: serenity::client::Context,
	pub options: CommOptions,
	pub msg: Option<Message>,
	pub inter: Option<ApplicationCommandInteraction>,
	pub author: serenity::model::user::User,
	pub member: Option<Member>,
	pub channel_id: serenity::model::id::ChannelId,
	pub guild_id: Option<serenity::model::id::GuildId>
}

impl CommandParams {
	/// doir stands for "delete original interaction response"
	pub async fn doir(&self) {
		if let Some(inter) = &self.inter {
			inter.delete_original_interaction_response(self.ctx.http.as_ref()).await.ok();
		}
	}
	pub async fn follow_up<'a, F>(&self, inter_res_fn: F) -> serenity::Result<Message>
		where
			for<'b> F: FnOnce(
				&'b mut CreateInteractionResponseFollowup<'a>,
			) -> &'b mut CreateInteractionResponseFollowup<'a>
	{
		if let Some(inter) = &self.inter {
			inter.create_followup_message(self.ctx.http.as_ref(), inter_res_fn).await
		}
		else {
			let mut inter_res = CreateInteractionResponseFollowup::default();
			inter_res_fn(&mut inter_res);
			let message = CreateMessage(inter_res.0, None, inter_res.1);
			self.channel_id.send_message(self.ctx.http.as_ref(), |new_msg| {
				*new_msg = message;
				new_msg
			}).await
		}
	}
}

#[derive(Debug)]
pub enum CommRes<'a> {
	None,
	Text(String),
	Msg(CreateMessage<'a>)
}

pub enum CommErr {
	Error(String, String),
	UnknownError,
	SyntaxError,
	UnknownCommand
}

pub struct Command<'a> {
	pub names: Vec<String>,
	pub desc: String,
	pub options: Vec<fn(&mut CreateApplicationCommandOption) -> &mut CreateApplicationCommandOption>,
	pub cat: String,
	pub func: fn(params: CommandParams) -> Pin<Box<dyn Future<Output = Result<CommRes<'a>, CommErr>> + std::marker::Send>>,
	pub args: String
}

impl Command<'_> {
	pub fn new() -> Self {
		Self {
			names: Vec::new(),
			desc: String::new(),
			options: Vec::new(),
			cat: String::new(),
			func: |_| Box::pin(async { Ok(CommRes::None) }),
			args: String::new()
		}
	}
	pub async fn run(&self, params: CommandParams) -> Result<CommRes, CommErr> {
		(self.func)(params).await
	}
}

pub struct ChloeManager<'a> {
	pub config: Value,
	pub commands: Vec<Command<'a>>
}

impl<'a> ChloeManager<'a> {
	pub fn new(config: Option<&str>, commands: Vec<Command<'a>>) -> Result<Self, String> {
		Ok(Self {
			config: if let Some(p) = config {
				match parse_config_file(p) {
					Ok(v) => v,
					Err(e) => return Err(e)
				}
			}
			else { serde_json::json!({}) },
			commands: commands
		})
	}
	pub fn command(&self, name: &str) -> Option<&Command<'a>> {
		for command_ in self.commands.iter() {
			for name_ in command_.names.iter() {
				if name_ == name {
					return Some(command_);
				}
			}
		}
		None
	}
	pub fn command_from_msg(&self, content: &str, prefix: &str) -> Option<&Command<'a>> {
		if let Some(arg_str) = content.strip_prefix(prefix) {
			let args: Vec<&str> = arg_str.split(' ').collect();
			return self.command(args[0]);
		}
		else {
			None
		}
	}
	pub async fn run_command(&self, name: &str, params: CommandParams) -> Option<Result<CommRes, CommErr>> {
		match self.command(name) {
			Some(command) => Some(command.run(params).await),
			None => None
		}
	}
	pub async fn process_msg(&self, msg: Message, ctx: serenity::client::Context, db: Arc<Mutex<Database>>, prefix: &str) -> Option<Result<(), CommErr>> {
		let content = msg.content.clone();
		if content.len() <= prefix.len() || prefix != &content.as_str()[0..prefix.len()].to_lowercase() {
			return None;
		}
		let arg_str = &content.as_str()[prefix.len()..];
		let args: Vec<&str> = arg_str.split(' ').collect();
		let member = match msg.member {
			Some(..) => {
				msg.guild_id.clone().unwrap().member(ctx.http.as_ref(), msg.author.id).await.ok()
			},
			None => None
		};
		let author = msg.author.clone();
		let channel_id = msg.channel_id.clone();
		let guild_id = msg.guild_id.clone();
		match self.run_command(args[0], CommandParams {
			prefix: prefix.to_string(),
			db: db,
			ctx: ctx.clone(),
			options: CommOptions(HashMap::new()),
			msg: Some(msg),
			inter: None,
			author: author,
			member: member,
			channel_id: channel_id.clone(),
			guild_id: guild_id
		}).await {
			Some(v) => Some(match v {
				Ok(v) => match v {
					CommRes::Text(text) => match channel_id.say(ctx.http, text).await {
						Ok(..) => Ok(()),
						Err(e) => Err(CommErr::Error(String::new(), format!("{e}")))
					},
					CommRes::Msg(msg) => match channel_id.send_message(ctx.http.as_ref(), |m| { *m = msg; m }).await {
						Ok(..) => Ok(()),
						Err(e) => Err(CommErr::Error(String::new(), format!("{e}")))
					},
					_ => Ok(())
				},
				Err(e) => {
					channel_id.send_message(ctx.http.as_ref(), |m| {
						let command = self.command(args[0]).unwrap();
						match &e {
							CommErr::Error(e1, e2) => {
								if !e2.is_empty() {
									eprintln!("Error processing message: {}", e2);
								}
								m.add_embed(|e| {
									let e = e.title("Error")
									.color(CHLOE.config["bad_color"].as_i64().unwrap() as i32);
									if e1.is_empty() {
										e.description("An error has occurred")
									}
									else {
										e.description(e1)
									}
								})
							},
							CommErr::UnknownError => m.add_embed(|e| {
								e.title("Error")
								.color(CHLOE.config["bad_color"].as_i64().unwrap() as i32)
								.description("An error has occurred")
							}),
							CommErr::SyntaxError => 
								m.add_embed(|e| {
								e.title(format!("{}{}", prefix, command.names[0]))
								.description(command.desc.clone())
								.fields(vec![("Syntax", format!("{}{} {}", prefix, command.names[0], command.args), false)])
								.color(CHLOE.config["embed_color"].as_i64().unwrap() as i32)
							}),
							CommErr::UnknownCommand => m.content("Unknown command")
						}
					}).await.ok();
					Err(e)
				}
			}),
			None => {
				channel_id.say(ctx.http.as_ref(), "Unknown command").await.ok();
				Some(Err(CommErr::UnknownCommand))
			}
		}
	}
	pub async fn process_inter(&self, inter: ApplicationCommandInteraction, ctx: serenity::client::Context, db: Arc<Mutex<Database>>) -> Option<Result<(), CommErr>> {
		inter.defer(ctx.http.as_ref()).await.unwrap();
		let member = match inter.member {
			Some(..) => {
				inter.guild_id.clone().unwrap().member(ctx.http.as_ref(), inter.user.id).await.ok()
			},
			None => None
		};
		let author = inter.user.clone();
		let channel_id = inter.channel_id.clone();
		let guild_id = inter.guild_id.clone();
		match CHLOE.run_command(inter.data.name.clone().as_str(), CommandParams {
			prefix: "/".to_string(),
			db: db,
			ctx: ctx.clone(),
			options: CommOptions::new(inter.data.options.clone()),
			msg: None,
			inter: Some(inter.clone()),
			author: author,
			member: member,
			channel_id: channel_id,
			guild_id: guild_id
		}).await {
			Some(v) => Some(match v {
				Ok(v) => {
					match v {
						CommRes::Text(text) => match inter.create_followup_message(ctx.http.as_ref(), |m| { m.content(text) }).await {
							Ok(..) => Ok(()),
							Err(e) => Err(CommErr::Error(String::new(), format!("{e}")))
						},
						CommRes::Msg(msg) => match inter.create_followup_message(ctx.http.as_ref(), |m| { *m = CreateInteractionResponseFollowup(msg.0, msg.2); m }).await {
							Ok(..) => Ok(()),
							Err(e) => Err(CommErr::Error(String::new(), format!("{e}")))
						},
						CommRes::None => {
							Ok(())
						}
					}
				},
				Err(e) => {
						inter.create_followup_message(ctx.http.as_ref(), |m| {
							let prefix = "/";
							let command = self.command(inter.data.name.clone().as_str()).unwrap();
							match &e {
								CommErr::Error(e1, e2) => {
									if !e2.is_empty() {
										eprintln!("Error processing message: {}", e2);
									}
									m.embed(|e| {
										let e = e.title("Error")
										.color(CHLOE.config["bad_color"].as_i64().unwrap() as i32);
										if e1.is_empty() {
											e.description("An error has occurred")
										}
										else {
											e.description(e1)
										}
									})
								},
								CommErr::UnknownError => m.embed(|e| {
									e.title("Error")
									.color(CHLOE.config["bad_color"].as_i64().unwrap() as i32)
									.description("An error has occurred")
								}),
								CommErr::SyntaxError => 
									m.embed(|e| {
									e.title(format!("{}{}", prefix, command.names[0]))
									.description(command.desc.clone())
									.fields(vec![("Syntax", format!("{}{} {}", prefix, command.names[0], command.args), false)])
									.color(CHLOE.config["embed_color"].as_i64().unwrap() as i32)
								}),
								CommErr::UnknownCommand => m.content("Unknown command")
							}
						}).await.ok();
						Err(e)
					}
			}),
			None => {
				inter.create_followup_message(ctx.http.as_ref(), |m| {
					m.content("Unknown command")
				}).await.ok();
				Some(Err(CommErr::UnknownCommand))
			}
		}
	}
}

pub fn parse_config_file(config_path_str: &str) -> Result<Value, String> {
	match {
		match fs::File::open(config_path_str) {
			Ok(mut config_file) => {
				let mut config_file_content = String::new();
				config_file.read_to_string(&mut config_file_content).unwrap();
				match json5::from_str(config_file_content.as_str()) {
					Ok(v) => Ok(v),
					Err(e) => Err(format!("Unable to parse the config file \"{config_path_str}\" as JSON5 or JSON: {e}"))
				}
			},
			Err(e) => Err(format!("Unable to open the config file \"{config_path_str}\": {e}"))
		}
	} {
		Ok(v) => Ok(v),
		Err(e) => {
			let config_path = Path::new(config_path_str);
			let def_config_path = config_path.with_extension("def.json5");
			if let Some(def_config_path_str) = def_config_path.to_str() {
				if def_config_path.exists() {
					if config_path.exists() {
						return Err(format!("{e}.         The example config file \"{def_config_path_str}\" might come in handy"));
					}
					else {
						println!("The config file \"{config_path_str}\" doesn't exist. I will attempt to copy the example config file \"{def_config_path_str}\" to \"{config_path_str}\".");
						if let Err(e) = fs::copy(def_config_path_str, config_path_str) {
							return Err(format!("Unable to copy \"{def_config_path_str}\" to \"{config_path_str}\": {e}"));
						}
						println!("Done. You can modify the config file \"{config_path_str}\" as you like.");
						return parse_config_file(config_path_str);
					}
				}
			}
			return Err(e);
		}
	}
}

pub fn disable_all_components(c: &mut CreateComponents) {
	for row in c.0.iter_mut() {
		for component in row["components"].as_array_mut().unwrap().iter_mut() {
			if component["type"].as_u64().unwrap() == 2 {
				component["disabled"] = serde_json::json!(true);
			}
		}
	}
}

pub fn make_message<'a, F>(msg_fn: F) -> CreateMessage<'a>
	where
		for<'b> F: FnOnce(
			&'b mut CreateMessage<'a>,
		) -> &'b mut CreateMessage<'a>
{
	let mut msg = CreateMessage::default();
	msg_fn(&mut msg);
	msg
}
