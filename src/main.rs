use std::{collections::HashMap, fs::{self, File}, io::Read};
use tokio::sync::Mutex;
use dotenv::dotenv;
use poise::serenity_prelude::{self as serenity, UserId};

use std::collections::hash_map::Entry;

type SwearCounterMap = HashMap<UserId, u32>;
type PersonalSwearList = HashMap<UserId, Vec<String>>;

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
    default_swear_list: Vec<String>,
    swear_lists: Mutex<PersonalSwearList>,
    swear_counters: Mutex<SwearCounterMap>,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    _data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            println!("Logged in as {}", data_about_bot.user.name)
        }
        serenity::FullEvent::Message { new_message } => {
            println!("{}", new_message.content);
            if new_message.content.to_lowercase().contains("fuck") {
                let response = format!("Hey {}, don't say bad words.", new_message.author_nick(ctx).await.unwrap_or(new_message.author.name.clone()));
                new_message.reply_mention(ctx, response).await?;
                // new_message.reply(ctx, response).await?;
            }
        }
        _ =>{} 
    };
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
        },
        _ => { },
    }
    drop(swear_lists);
    if inserted {
        ctx.reply("you already are being tracked by the swear jar!").await.unwrap();
    };
        // .or_insert_with(|| ctx.data().default_swear_list.clone());
    Ok(())
    // if let Some(_) = ctx.data().swear_lists.lock().unwrap().get(curr_user) {
    //     ctx.reply("you already are being tracked by the swear jar!").await?;
    // } else {
    //     ctx.data().swear_lists.lock().unwrap().insert(*curr_user, ctx.data().default_swear_list.clone());
    // }
    // Ok(())
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

    let default_swear_list: Vec<String> = fs::read_to_string("../default_swears.txt").unwrap().split('\n').map(|s| s.to_string()).collect();

    // let saved_swear_counters: HashMap<UserId, u32> =

    let intents = serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions { 
            commands: vec![age()],
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
                    swear_counters: Mutex::new(HashMap::new())
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}