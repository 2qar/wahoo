use std::collections::HashSet;

use serenity::prelude::Context;
use serenity::model::id::UserId;
use serenity::model::channel::Message;
use serenity::framework::standard::{Args, HelpOptions, CommandGroup, CommandResult};
use serenity::framework::standard::macros::help;

#[help]
pub fn help(ctx: &mut Context, msg: &Message, mut args: Args, _opt: &'static HelpOptions, groups: &[&'static CommandGroup], _ids: HashSet<UserId>) -> CommandResult {
    let help_msg = match args.single::<String>() {
        Ok(s) => formatted_command(groups, s),
        Err(_) => help_str(groups),
    };

    if let Err(why) = msg.channel_id.say(&ctx.http, help_msg) {
        eprintln!("error sending help message: {}", why);
    }

    Ok(())
}

fn help_str(groups: &[&'static CommandGroup]) -> String {
    let mut m = String::from("__**Commands**__\nTo get help with a command, pass its name as an argument to this command.\n");
    for group in groups {
        m.push_str(formatted_group(&group).as_str());
    }
    m
}

fn formatted_group(group: &CommandGroup) -> String {
    let mut commands = String::new();
    let last_index = group.options.commands.len() - 1;
    for (i, cmd) in group.options.commands.iter().enumerate() {
        commands.push_str(format!("`{}`", cmd.options.names[0]).as_str());
        if i != last_index {
            commands.push_str(", ");
        }
    }
    format!("**{}**: {}\n", group.name, commands)
}

fn formatted_command(groups: &[&'static CommandGroup], mut s: String) -> String {
    s = s.to_lowercase();
    for group in groups {
        for command in group.options.commands {
            for &name in command.options.names {
                if s == name {
                    return format!("__**{}**__\n**Usage**: `{}`\n**Description**: {}",
                                   name, command.options.usage.unwrap(), command.options.desc.unwrap());
                }
            }
        }
    }

    String::from("No command found.")
}
