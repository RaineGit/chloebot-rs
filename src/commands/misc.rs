use super::*;

pub fn commands<'a>() -> Vec<Command<'a>> {
	let category = "Misc".to_string();
	vec![
		// ping
		Command {
			names: svec!["ping"],
			desc: "Check if I am online and working".to_string(),
			options: Vec::new(),
			cat: category.clone(),
			func: |_params: CommandParams| func!({
				Ok(CommRes::Text("pong".to_string()))
			}),
			..Command::new()
		},
		// pong
		Command {
			names: svec!["pong"],
			desc: "Command for testing the database".to_string(),
			options: Vec::new(),
			cat: category.clone(),
			func: |params: CommandParams| func!({
				let count = {
					let db = &mut params.db.lock().unwrap();
					let count = match db.get(&["pings"]).unwrap() {
						Value::Number(num) => num.as_u64().unwrap(),
						_ => 0
					} + 1;
					db.set(&["pings"], count.into()).unwrap();
					count
				};
				Ok(CommRes::Text(format!("ponged {} times", count)))
			}),
			..Command::new()
		},
		// help
		Command {
			names: svec!["help"],
			desc: "Find out what commands I have".to_string(),
			options: Vec::new(),
			cat: category.clone(),
			func: |params: CommandParams| func!({
				let mut row = CreateActionRow::default();
				for category in CHLOE.config["categories"].as_array().unwrap().iter() {
					let category = match category {
						Value::String(s) => s.clone(),
						_ => String::new()
					};
					let mut button = CreateButton::default();
					button.custom_id(category.clone())
						.label(category)
						.style(ButtonStyle::Secondary);
					row.add_button(button);
				}
				let mut components = CreateComponents::default();
				components.add_action_row(row);
				let mut m = handle!(params.follow_up(|m| {
					m.content("Choose a category")
					.set_components(components.clone())
				}).await);
				let mut mci = m.await_component_interactions(&params.ctx).timeout(Duration::from_secs(10)).build();
				while let Some(mci) = mci.next().await {
					handle!(mci.create_interaction_response(&params.ctx, |r| {
						r.kind(InteractionResponseType::ChannelMessageWithSource).interaction_response_data(
							|d| {
								let mut fields: Vec<(String, &str, bool)> = Vec::new();
								for command in CHLOE.commands.iter() {
									if command.cat == mci.data.custom_id {
										fields.push((
											format!("{}{} {}", params.prefix, command.names[0], command.args),
											if command.desc.is_empty() { "." } else { command.desc.as_str() },
											false
										));
									}
								}
								d.embed(|e| {
									e.title(format!("{} commands", mci.data.custom_id))
									.description("<> = Required field\n[] = Optional field")
									.fields(fields)
									.color(CHLOE.config["embed_color"].as_i64().unwrap() as i32)
								})
							}
						)
					})
					.await);
				}
				handle!(m.edit(&params.ctx.http, |m| {
					disable_all_components(&mut components);
					m.set_components(components)
				}).await);
				Ok(CommRes::None)
			}),
			..Command::new()
		},
		// invite
		Command {
			names: svec!["invite"],
			desc: "Invite me to other servers".to_string(),
			options: Vec::new(),
			cat: category.clone(),
			func: |_params: CommandParams| func!({
				match &CHLOE.config["invite"] {
					Value::String(invite) => Ok(CommRes::Text(format!("Thank you!\n{}", invite))),
					Value::Null => Err(error!(, "The entry \"invite\" doesn't exist in the config")),
					_ => Err(error!(, "The entry \"invite\" in the config must be a string"))
				}
			}),
			..Command::new()
		},
		// say
		Command {
			names: svec!["say"],
			desc: "Make me say stuff".to_string(),
			options: vec![|option| {
				option.name("text").kind(CommandOptionType::String).required(true)
					.description("Text that I must say")
			}],
			cat: category.clone(),
			func: |params: CommandParams| func!({
				let text = handle_syntax_opt!(params.options.get_string("text"));
				Ok(CommRes::Text(text))
			}),
			..Command::new()
		},
		// error
		Command {
			names: svec!["error"],
			desc: "Cause an error".to_string(),
			options: vec![|option| {
				option.name("text").kind(CommandOptionType::String).required(true)
					.description("Error message")
			}],
			cat: category.clone(),
			func: |params: CommandParams| func!({
				let text = handle_syntax_opt!(params.options.get_string("text"));
				Err(error!(text))
			}),
			..Command::new()
		},
		// love
		Command {
			names: svec!["love"],
			desc: "Love someone".to_string(),
			options: vec![|option| {
				option.name("who").kind(CommandOptionType::User).required(true)
					.description("User you want to love")
			}],
			cat: category.clone(),
			func: |params: CommandParams| func!({
				let who = handle_syntax_opt!(params.options.get_user("who"));
				Ok(CommRes::Text(format!("{} loves {} :two_hearts:", params.author.name, who.0.name)))
			}),
			..Command::new()
		},
	]
}