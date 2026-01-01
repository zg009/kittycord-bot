use std::{collections::{HashMap, hash_map::OccupiedEntry}, fs::{self, File}, io::Read, ops::Add};
use tokio::sync::Mutex;
use dotenv::dotenv;
use poise::{CreateReply, ReplyHandle, serenity_prelude::{self as serenity, CreateAttachment, MessageBuilder, UserId}};
use regex::Regex;

use std::collections::hash_map::Entry;


type SwearCounterMap = HashMap<UserId, u32>;
type PersonalSwearList = HashMap<UserId, Vec<Regex>>;
type PointsMap = HashMap<UserId, u32>;
type DailyRedeem = HashMap<UserId, time::OffsetDateTime>;
type MainRng = Arc<Mutex<ThreadRng>>;

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
    user_points: Mutex<PointsMap>,
    user_redeem_time: Mutex<DailyRedeem>,
    // callable_rng: Arc<Mutex<ThreadRng>>
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
            let mut sc = data.swear_counters.lock().await;
            sc.entry(new_message.author.id).and_modify(|e| *e += result);
            println!("{} added {} swears", &new_message.author.id, result);
            match sc.get(&new_message.author.id) {
                Some(v) => println!("{} is at {} swears", &new_message.author.id, &v),
                None => {},
            }
        }
        _ =>{} 
    };
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn six_seven(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let rep = ctx.say("67").await;
    reply_handler(&rep)
}

#[poise::command(slash_command, prefix_command)]
async fn add_swear_regex(
    ctx: Context<'_>,
    #[description="The swear regex string you want to add."] swear: String
) -> Result<(), Error> {
    let curr_user = &ctx.author().id;
    let mut swear_lists = ctx.data().swear_lists.lock().await;
    swear_lists.entry(*curr_user).and_modify(|e| {
        let regex = Regex::new(&swear);
        match regex {
            Ok(r) => e.push(r),
            Err(e) => { println!("{:#?}", e) },
        }
    });
    let res = ctx.reply(format!("you added {} to your swear jar {}.", swear, curr_user)).await;
    reply_handler(&res)
}

#[poise::command(slash_command, prefix_command)]
async fn add_swear_string(
    ctx: Context<'_>,
    #[description="The swear string you want to add."] swear: String,
) -> Result<(), Error> {
    let curr_user = &ctx.author().id;
    let mut swear_lists = ctx.data().swear_lists.lock().await;
    swear_lists.entry(*curr_user).and_modify(|e| {
        let regex = Regex::new(&format!("^{}$", swear.trim()));
        match regex {
            Ok(r) => e.push(r),
            Err(e) => { println!("{:#?}", e) },
        }
    });
    let res = ctx.reply(format!("you added {} to your swear jar {}.", swear.trim(), curr_user)).await;
    reply_handler(&res)
}

#[poise::command(slash_command, prefix_command)]
async fn quit_swear_jar(
    ctx: Context<'_>
) -> Result<(), Error> {
    let curr_user = &ctx.author().id;
    let mut swear_lists = ctx.data().swear_lists.lock().await;
    
    match swear_lists.entry(*curr_user) {
        Entry::Vacant(_) => {
            ctx.reply(format!("you are not being tracked by the swear jar {}", ctx.author())).await.unwrap();
        },
        Entry::Occupied(oe) => { 
            oe.remove_entry();
            let mut sc = ctx.data().swear_counters.lock().await;
            let c = sc.get(curr_user).cloned();
            let swear_counter = sc.entry(*curr_user);        
            match swear_counter {
                Entry::Occupied(oe2) => {
                    oe2.remove_entry();
                    ctx.reply(format!("you swore {} times before giving up {}.\nyou have deleted your swear jar.", c.unwrap(), ctx.author())).await.unwrap()
                },
                Entry::Vacant(_) => ctx.reply(format!("you haven't sworn yet {}", ctx.author())).await.unwrap(),
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
    let res = ctx.reply(response).await;
    reply_handler(&res)
}

const RAT_LINK: &str = "./src/bigbellyrat.gif";
#[poise::command(slash_command, prefix_command)]
async fn big_belly_rat(ctx: Context<'_>) -> Result<(), Error> {
    let attachment = CreateAttachment::path(RAT_LINK).await.unwrap();
    let builder = CreateReply::default().attachment(attachment);
    let res = ctx.send(builder).await;
    reply_handler(&res)
}

fn reply_handler(res: &Result<ReplyHandle<'_>, serenity::Error>) -> Result<(), Error> {
    match res {
        Ok(_) => Ok(()),
        Err(e) => {
            println!("{:#?}", e);
            Ok(())
        }
    }
}

#[poise::command(slash_command, prefix_command)]
async fn kill_dan(
    ctx: Context<'_>
) -> Result<(), Error> {
    let res = ctx.reply(format!("{} has killed dan", ctx.author())).await;
    reply_handler(&res)
}

#[poise::command(slash_command, prefix_command)]
async fn request_twenty_dollars(
    ctx: Context<'_>
) -> Result<(), Error> {
    let res = ctx.reply("here is $20 real dollars").await;
    reply_handler(&res)
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
    let mut swear_counter = ctx.data().swear_counters.lock().await;
    let mut inserted = true;
    match swear_lists.entry(*curr_user) {
        Entry::Vacant(_) => {
            inserted = false;
            swear_lists.insert(*curr_user, ctx.data().default_swear_list.clone());
            swear_counter.insert(*curr_user, 0);
            ctx.reply(format!("swear jar created for {}", ctx.author())).await.unwrap();
        },
        _ => { },
    }
    if inserted {
        ctx.reply("you already are being tracked by the swear jar!").await.unwrap();
    };
    Ok(())
}

use rand::{prelude::*};

#[poise::command(slash_command, prefix_command)]
async fn gamble(
    ctx: Context<'_>,
    #[description = "amount of points to gamble away"] points: u32
) -> Result<(), Error> {
    let author_name = ctx.author().name.clone();
    let caller = ctx.author().id;
    let mut user_points = ctx.data().user_points.lock().await;
    match user_points.entry(caller) {
        Entry::Occupied(occupied_entry) => {
            let curr_points = *occupied_entry.get();
            
            if points <= curr_points {
                // let rand_num = rng.random::<u32>();
            } else {
                ctx.reply(format!("{}, you cannot gamble {} points when you only have {} points.", author_name, points, curr_points)).await.unwrap();
            }
        },
        Entry::Vacant(_) => {
            ctx.reply(format!("{}, you have no points to gamble! have you done your daily redeem?", author_name)).await.unwrap();
        },
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
async fn daily_reward(ctx: Context<'_>) -> Result<(), Error> {
    // get the function caller
    let caller = ctx.author().id;
    // are User objects cheap to clone?
    let author_name = ctx.author().name.clone();
    // get the points struct
    let mut user_points = ctx.data().user_points.lock().await;
    // get their last redeem
    let mut user_redeem = ctx.data().user_redeem_time.lock().await;
    // logic to determine whether they can redeem it
    let can_redeem = match user_redeem.entry(caller) {
        // if there is already a previous time entry
        Entry::Occupied(mut occupied_entry) => {
            let last_time = occupied_entry.get();
            let curr_time = time::OffsetDateTime::now_utc();
            let res = *last_time - curr_time;
            // if it has been equal or greater than 24 utc hours
            if res.whole_hours() >= 24 {
                // update the time in the struct
                occupied_entry.insert(curr_time);
                // they can redeem
                true
            // not been more than 24 hours
            } else {
                // can't redeem, don't update
                false
            }
        },
        // first redeem
        Entry::Vacant(vacant_entry) => {
            // record redemption time
            vacant_entry.insert(time::OffsetDateTime::now_utc());
            // can redeem
            true
        },
    };
    // get the points
    match user_points.entry(caller) {
        // if they are already stocking points
        Entry::Occupied(mut occupied_entry) => {
            if can_redeem {
                let new_points = occupied_entry.get_mut().add(100);
                ctx.reply(format!("{} you now have {} points.", author_name, new_points)).await.unwrap();
            } else {
                ctx.reply(format!("{} you already redeemed for today.", author_name)).await.unwrap();
            }
        },
        // if they cannot be found, it should be their first redeem, no can_redeem check needed
        Entry::Vacant(vacant_entry) => {
            let new_points = vacant_entry.insert(100);
            ctx.reply(format!("{} you now have {} points.", author_name, new_points)).await.unwrap();
        },
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

    let  framework = poise::Framework::builder()
        .options(poise::FrameworkOptions { 
            commands: vec![age(), create_swear_jar(), add_swear_regex(), add_swear_string(), 
                quit_swear_jar(), 
                big_belly_rat(), 
                daily_reward(),
                gamble(),
                zap(), 
                six_seven(), 
                request_twenty_dollars(), 
                public_shame(), 
                kill_dan()],
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
                    swear_counters: Mutex::new(saved_swear_counters),
                    user_points: Mutex::new(HashMap::new()),
                    user_redeem_time: Mutex::new(HashMap::new()),
                })
            })
        })
        .build();

        framework.options().commands.iter().for_each(|command| println!("{}", command.name));
    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}