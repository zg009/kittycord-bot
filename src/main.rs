use std::{collections::HashMap, fs::{self, File}, io::Read};
use tokio::sync::Mutex;
use dotenv::dotenv;
use poise::serenity_prelude::{self as serenity, MessageBuilder, User, UserId};
use regex::Regex;

use std::collections::hash_map::Entry;

type SwearCounterMap = HashMap<UserId, u32>;
type PersonalSwearList = HashMap<UserId, Vec<Regex>>;

fn write_scm_to_file(swear_counter_map: &SwearCounterMap, file_path: &str) -> Result<(), Error> {
    let file = File::create(file_path).expect("path for swear counter map file could not be found");
    match serde_json::to_writer(file, swear_counter_map) {
        Ok(_) => Ok(()),
        Err(e) => Err(Box::new(e)) 
    }
}

fn read_scm_from_file(path: &str) -> SwearCounterMap {
    let mut file = File::open(path).expect("file to read scm does not exist");
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    serde_json::from_str(&contents).expect("file data could not be converted to scm")
}

struct Data {
    default_swear_list: Vec<Regex>,
    swear_lists: Mutex<PersonalSwearList>,
    swear_counters: Mutex<SwearCounterMap>,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

async fn event_handler(
    _ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            println!("Logged in as {}", data_about_bot.user.name)
        }
        serenity::FullEvent::Message { new_message } => {
            println!("{}", new_message.content);
            let unlocked = data.swear_lists.lock().await;
            let swear_regexes = unlocked.get(&new_message.author.id);
            println!("{} swears", swear_regexes.iter().len());
            let content = &new_message.content.to_lowercase();
            println!("message content: {}", content);
            
            let result = match swear_regexes {
                Some(regexes) => regexes.iter().map(|re| {
                    println!("{} is match: {}", re.as_str(), re.is_match(content));
                    if re.is_match(content) { 
                        return 1; 
                    } 
                    else { 
                        return 0; 
                    }
                }).sum(),
                None => { 0 },
            };
            println!("{} count of result", result);
            data.swear_counters.lock().await.entry(new_message.author.id).and_modify(|e| *e += result);
            println!("{} added {} swears", &new_message.author.id, result);
        }
        _ =>{} 
    };
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn six_seven(
    ctx: Context<'_>,
) -> Result<(), Error> {
    ctx.say("67").await.unwrap();
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn add_swear_regex(
    ctx: Context<'_>,
    #[description="The swear regex string you want to add."] swear: String
) -> Result<(), Error> {
    let curr_user = &ctx.author().id;
    let mut swear_lists = ctx.data().swear_lists.lock().await;
    swear_lists.entry(*curr_user).and_modify(|e| {
        e.push(Regex::new(&swear).unwrap());
    });
    ctx.reply(format!("you added {} to your swear jar {}.", swear, curr_user)).await.unwrap();
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn add_swear_string(
    ctx: Context<'_>,
    #[description="The swear string you want to add."] swear: String,
) -> Result<(), Error> {
    let curr_user = &ctx.author().id;
    let mut swear_lists = ctx.data().swear_lists.lock().await;
    swear_lists.entry(*curr_user).and_modify(|e| {
        e.push(Regex::new(&format!("^{}$", swear.trim())).unwrap())
    });
    ctx.reply(format!("you added {} to your swear jar {}.", swear.trim(), curr_user)).await.unwrap();
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn quit_swear_jar(
    ctx: Context<'_>
) -> Result<(), Error> {
    let curr_user = &ctx.author().id;
    let mut swear_lists = ctx.data().swear_lists.lock().await;
    let swear_counter = ctx.data().swear_counters.lock().await;
    match swear_lists.entry(*curr_user) {
        Entry::Vacant(_) => {
            ctx.reply(format!("you are not being tracked by the swear jar {}", ctx.author())).await.unwrap();
        },
        Entry::Occupied(oe) => { 
            oe.remove_entry();
            match swear_counter.get(curr_user) {
                Some(c) => ctx.reply(format!("you swore {} times before giving up {}.\nyou have deleted your swear jar.", c, ctx.author())).await.unwrap(),
                None => ctx.reply(format!("you haven't sworn yet {}", ctx.author())).await.unwrap(),
            };
        },
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn zap(
    ctx: Context<'_>,
    #[description="User to zap."] user: serenity::User,
) -> Result<(), Error> {
    let response = MessageBuilder::new()
        .user(ctx.author())
        .push(" has zapped ")
        .user(user)
        .push(".")
        .build();
    ctx.reply(response).await.unwrap();
    Ok(())
}


#[poise::command(slash_command, prefix_command)]
async fn kill_dan(
    ctx: Context<'_>
) -> Result<(), Error> {
    ctx.reply(format!("{} has killed dan", ctx.author())).await.unwrap();
    Ok(())
}


#[poise::command(slash_command, prefix_command)]
async fn request_twenty_dollars(
    ctx: Context<'_>
) -> Result<(), Error> {
    ctx.reply("here is $20 real dollars").await.unwrap();
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn public_shame(
    ctx: Context<'_>,
    #[description="User you want to publicly shame"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let curr_user = &ctx.author().id;
    let mut swear_lists = ctx.data().swear_lists.lock().await;
    let swear_counter = ctx.data().swear_counters.lock().await;
    match user {
        Some(usr) => match swear_lists.entry(usr.id) {
            Entry::Vacant(_) => {
                ctx.reply(format!("{} is not being tracked by the swear jar {}", usr, ctx.author())).await.unwrap();
            },
            Entry::Occupied(_) => { 
                match swear_counter.get(&usr.id) {
                    Some(c) if *c == 0 => ctx.reply(format!("{} hasn't sworn yet {}", usr, ctx.author())).await.unwrap(), 
                    Some(c) => ctx.reply(format!("{} has sworn {} times {}.", usr, c, ctx.author())).await.unwrap(),
                    None => ctx.reply(format!("{} hasn't sworn yet {}", usr, ctx.author())).await.unwrap(),
                };
            },
        },
        None => match swear_lists.entry(*curr_user) {
            Entry::Vacant(_) => {
                ctx.reply(format!("you are not being tracked by the swear jar {}", ctx.author())).await.unwrap();
            },
            Entry::Occupied(_) => { 
                match swear_counter.get(curr_user) {
                    Some(c) if *c == 0 => ctx.reply(format!("you haven't sworn yet {}", ctx.author())).await.unwrap(), 
                    Some(c) => ctx.reply(format!("you have sworn {} times {}.", c, ctx.author())).await.unwrap(),
                    None => ctx.reply(format!("you haven't sworn yet {}", ctx.author())).await.unwrap(),
                };
            },
        }
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn create_swear_jar(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let curr_user = &ctx.author().id;
    let mut swear_lists = ctx.data().swear_lists.lock().await;
    let mut inserted = true;
    match swear_lists.entry(*curr_user) {
        Entry::Vacant(_) => {
            inserted = false;
            swear_lists.insert(*curr_user, ctx.data().default_swear_list.clone());
            ctx.reply(format!("swear jar created for {}", ctx.author())).await.unwrap();
        },
        _ => { },
    }
    if inserted {
        ctx.reply("you already are being tracked by the swear jar!").await.unwrap();
    };
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn age(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!("{}'s account was created at {}", u.name, u.created_at());
    ctx.say(response).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().expect("no dotenv file found");
    let token = std::env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN in env file");

    let default_swear_list: Vec<Regex> = fs::read_to_string("default_swears.txt")
        .expect("could not find default swears file")
        .split_whitespace()
        .map(|s| Regex::new(s.trim()).unwrap())
        .collect();
    default_swear_list.iter().for_each(|r| println!("swear regex : {}", r.as_str()));
    let saved_swear_counters: HashMap<UserId, u32> = match fs::read_to_string("saved_swear_counters.txt")  {
        Ok(s) => serde_json::from_str(&s).expect("file could not be parsed to hashmap"),
        Err(_) => HashMap::new(),
    };

    let intents = serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions { 
            commands: vec![age(), create_swear_jar()],
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            ..Default::default() 
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    default_swear_list: default_swear_list,
                    swear_lists: Mutex::new(HashMap::new()),
                    swear_counters: Mutex::new(saved_swear_counters)
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}