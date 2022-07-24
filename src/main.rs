use db::Database;
use std::env;
use std::sync::{Mutex, Arc};
use serenity::async_trait;
use serenity::prelude::*;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::application::interaction::Interaction;
use serenity::builder::CreateInteractionResponseFollowup;
use lazy_static::lazy_static;
use serde_json::Value;
mod commands;
pub mod core;
use crate::core::*;

lazy_static! {
    pub static ref CHLOE: ChloeManager<'static> = ChloeManager::new(
		Some("config.json5"),
		commands::commands()
	).unwrap();
	static ref DB: Arc<Mutex<Database>> = Arc::new(Mutex::new(Database::open("db").unwrap()));
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
	async fn message(&self, ctx: Context, msg: Message) {
		if msg.author.bot {
			return;
		}
		let db = DB.clone();
		let prefix = match &CHLOE.config["prefix"] {
			Value::String(v) => v.as_str(),
			Value::Null => panic!("The entry \"prefix\" doesn't exist in the config"),
			_ => panic!("The entry \"prefix\" in the config must be a string")
		};
		if let Some(r) = CHLOE.process_msg(msg.clone(), ctx.clone(), db, prefix).await {
			if let Err(e) = r {
				//msg.channel_id.send_message(&ctx.http, ).await.ok();
			}
		}
	}

	async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
		if let Interaction::ApplicationCommand(command) = interaction {
			//println!("Received command interaction: {:#?}", command);
			command.defer(ctx.http.as_ref()).await.unwrap();
			//command.delete_original_interaction_response(ctx.http.as_ref());
			//command.channel_id.say(ctx.http.as_ref(), "pong").await.unwrap();
			let member = match command.member {
				Some(..) => {
					command.guild_id.clone().unwrap().member(ctx.http.as_ref(), command.user.id).await.ok()
				},
				None => None
			};
			let author = command.user.clone();
			let channel_id = command.channel_id.clone();
			let guild_id = command.guild_id.clone();
			match CHLOE.run_command(command.data.name.clone().as_str(), CommandParams {
				args: Vec::new(),
				arg_string: String::new(),
				prefix: "/".to_string(),
				db: DB.clone(),
				ctx: ctx.clone(),
				options: CommOptions::new(command.data.options.clone()).0,
				msg: None,
				inter: Some(command.clone()),
				author: author,
				member: member,
				channel_id: channel_id,
				guild_id: guild_id
			}).await {
				Some(v) => Some(match v {
					Ok(v) => {
						match v {
							CommRes::Text(text) => match command.create_followup_message(ctx.http.as_ref(), |m| { m.content(text) }).await {
								Ok(..) => Ok(()),
								Err(e) => Err(CommErr::Error(String::new(), format!("{e}")))
							},
							CommRes::Msg(msg) => match command.create_followup_message(ctx.http.as_ref(), |m| { *m = CreateInteractionResponseFollowup(msg.0, msg.2); m }).await {
								Ok(..) => Ok(()),
								Err(e) => Err(CommErr::Error(String::new(), format!("{e}")))
							},
							CommRes::None => {
								Ok(())
							}
						}
					},
					Err(e) => Err(e)
				}),
				None => None
			};
		}
	}

	async fn ready(&self, ctx: Context, ready: Ready) {
		let commands = &CHLOE.commands;
        serenity::model::application::command::Command::set_global_application_commands(&ctx.http, |new_commands| {
			for command in commands.iter() {
				new_commands.create_application_command(|new_command| {
                    new_command.name(command.names[0].clone()).description(command.desc.clone());
					for option in command.options.iter() {
						new_command.create_option(option);
					}
					new_command
                });
			}
			new_commands
		}).await.unwrap();
		println!("{} is connected!", ready.user.name);
	}
}

#[tokio::main]
async fn main() {
	{
		let _ = &CHLOE.config;
		let _ = DB.clone();
	}
	let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

	let intents = GatewayIntents::GUILD_MESSAGES
		| GatewayIntents::DIRECT_MESSAGES
		| GatewayIntents::MESSAGE_CONTENT;

	let mut client =
		Client::builder(&token, intents).event_handler(Handler).await.expect("Err creating client");

	if let Err(why) = client.start().await {
		println!("Client error: {:?}", why);
	}
}