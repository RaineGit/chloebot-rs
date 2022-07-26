use db::Database;
use std::env;
use std::sync::{Mutex, Arc};
use serenity::async_trait;
use serenity::prelude::*;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::application::interaction::Interaction;
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
		CHLOE.process_msg(msg.clone(), ctx.clone(), db, prefix).await;
	}

	async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
		if let Interaction::ApplicationCommand(command) = interaction {
			CHLOE.process_inter(command, ctx, DB.clone()).await;
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
	let token = env::var("DISCORD_TOKEN").expect("Expected a discord token in the environment variable DISCORD_TOKEN");

	let intents = GatewayIntents::GUILD_MESSAGES
		| GatewayIntents::DIRECT_MESSAGES
		| GatewayIntents::MESSAGE_CONTENT;

	let mut client =
		Client::builder(&token, intents).event_handler(Handler).await.expect("Err creating client");

	if let Err(why) = client.start().await {
		println!("Client error: {:?}", why);
	}
}
