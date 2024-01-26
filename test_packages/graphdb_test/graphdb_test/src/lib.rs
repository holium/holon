use serde::{Deserialize, Serialize};

use kinode_process_lib::{await_message, call_init, graphdb, println, Address, Message, Response};

wit_bindgen::generate!({
    path: "wit",
    world: "process",
    exports: {
        world: Component,
    },
});

#[derive(Debug, Serialize)]
struct Person<'a> {
    name: &'a str,
    company: &'a str,
}

///  Example usage:
///
///  /m our@graphdb_test:graphdb_test:template.os {"Open": {"db": "test_db"}}
///  /m our@graphdb_test:graphdb_test:template.os {"Read": {"db": "test_db", "query": "SELECT * FROM person;"}}
///
#[derive(Debug, Serialize, Deserialize)]
enum TestRequest {
    Init { db: String },
    Open { db: String },
    Read { db: String, query: String },
    Write { db: String, statement: String },
}

#[derive(Debug, Serialize, Deserialize)]
enum TestResponse {
    DbOpened,
    InitScriptRan,
}

fn handle_message(our: &Address) -> anyhow::Result<()> {
    let message = await_message()?;

    match message {
        Message::Response { .. } => {
            return Err(anyhow::anyhow!("unexpected Response: {:?}", message));
        }
        Message::Request {
            // ref source,
            ref body,
            ..
        } => {
            match serde_json::from_slice(body)? {
                // /m our@graphdb_test:graphdb_test:template.os {"Init": {"db": "test_db"}}
                TestRequest::Init { ref db }  => {
                    // multiline string
                    let init_script = r#"
                        BEGIN TRANSACTION;

                        -- categories
                        CREATE category:culture SET name = 'culture';
                        CREATE category:event SET name = 'event';
                        CREATE category:meme SET name = 'meme';
                        CREATE category:person SET name = 'person';
                        CREATE category:site SET name = 'site';
                        CREATE category:subculture SET name = 'subculture';

                        -- culture
                        CREATE culture:art SET name = 'art';
                        CREATE culture:country SET name = 'country';
                        CREATE culture:food SET name = 'food';
                        CREATE culture:movement SET name = 'movement';
                        CREATE culture:music SET name = 'music';
                        CREATE culture:religion SET name = 'religion';
                        CREATE culture:sport SET name = 'sport';
                        CREATE culture:technology SET name = 'technology';

                        -- events
                        CREATE event:auction SET name = 'Award Ceremony';
                        CREATE event:award_ceremony SET name = 'Award Ceremony';
                        CREATE event:campaign SET name = 'Campaign';
                        CREATE event:competition SET name = 'Competition';
                        CREATE event:conflict SET name = 'Conflict';
                        CREATE event:controversy SET name = 'Controversy';
                        CREATE event:convention SET name = 'Convention';
                        CREATE event:crime SET name = 'Crime';
                        CREATE event:disaster SET name = 'Disaster';
                        CREATE event:election SET name = 'Election';
                        CREATE event:flash_mob SET name = 'Flash Mob';
                        CREATE event:gaffe SET name = 'Gaffe';
                        CREATE event:hack SET name = 'Hack';
                        CREATE event:holiday SET name = 'Holiday';
                        CREATE event:law SET name = 'Law';
                        CREATE event:leak SET name = 'Leak';
                        CREATE event:performance SET name = 'Performance';
                        CREATE event:prank SET name = 'Prank';
                        CREATE event:promotion SET name = 'Promotion';
                        CREATE event:protest SET name = 'Protest';
                        CREATE event:raid SET name = 'Raid';
                        CREATE event:trial SET name = 'Trial';

                        -- memes
                        CREATE meme:ai_generated SET name = 'AI-generated';
                        CREATE meme:advertisement SET name = 'Advertisement';
                        CREATE meme:animal SET name = 'Animal';
                        CREATE meme:axiom SET name = 'Axiom';
                        CREATE meme:catchphrase SET name = 'Catchphrase';
                        CREATE meme:character SET name = 'Character';
                        CREATE meme:cliche SET name = 'Cliché';
                        CREATE meme:conspiracy SET name = 'Conspiracy Theory';
                        CREATE meme:copypasta SET name = 'Copypasta';
                        CREATE meme:creepypasta SET name = 'Creepypasta';
                        CREATE meme:dance SET name = 'Dance';
                        CREATE meme:emoticon SET name = 'Emoticon';
                        CREATE meme:exploitable SET name = 'Exploitable';
                        CREATE meme:fan_art SET name = 'Fan Art';
                        CREATE meme:fan_labor SET name = 'Fan Labor';
                        CREATE meme:hashtag SET name = 'Hashtag';
                        CREATE meme:hoax SET name = 'Hoax';
                        CREATE meme:image_macro SET name = 'Image Macro';
                        CREATE meme:lip_dub SET name = 'Lip Dub';
                        CREATE meme:parody SET name = 'Parody';
                        CREATE meme:participatory_media SET name = 'Participatory Media';
                        CREATE meme:photoshop SET name = 'Photoshop';
                        CREATE meme:pop_culture SET name = 'Pop Culture Reference';
                        CREATE meme:reaction SET name = 'Reaction';
                        CREATE meme:remix SET name = 'Remix';
                        CREATE meme:shock_media SET name = 'Shock Media';
                        CREATE meme:slang SET name = 'Slang';
                        CREATE meme:snowclone SET name = 'Snowclone';
                        CREATE meme:social_game SET name = 'Social Game';
                        CREATE meme:song SET name = 'Song';
                        CREATE meme:sound_effect SET name = 'Sound Effect';
                        CREATE meme:viral_debate SET name = 'Viral Debate';
                        CREATE meme:viral_video SET name = 'Viral Video';
                        CREATE meme:visual_effect SET name = 'Visual Effect';

                        -- people
                        CREATE person:activist SET name = 'Activist';
                        CREATE person:actor SET name = 'Actor';
                        CREATE person:artist SET name = 'Artist';
                        CREATE person:athlete SET name = 'Athlete';
                        CREATE person:businessperson SET name = 'Businessperson';
                        CREATE person:comedian SET name = 'Comedian';
                        CREATE person:creator SET name = 'Creator';
                        CREATE person:filmmaker SET name = 'Filmmaker';
                        CREATE person:gamer SET name = 'Gamer';
                        CREATE person:hacker SET name = 'Hacker';
                        CREATE person:historical_figure SET name = 'Historical Figure';
                        CREATE person:influencer SET name = 'Influencer';
                        CREATE person:model SET name = 'Model';
                        CREATE person:musician SET name = 'Musician';
                        CREATE person:organization SET name = 'Organization';
                        CREATE person:politician SET name = 'Politician';
                        CREATE person:programmer SET name = 'Programmer';
                        CREATE person:scientist SET name = 'Scientist';
                        CREATE person:streamer SET name = 'Streamer';
                        CREATE person:tv_personality SET name = 'TV Personality';
                        CREATE person:vlogger SET name = 'Vlogger';
                        CREATE person:writer SET name = 'Writer';

                        -- sites
                        CREATE site:application SET name = 'Application';
                        CREATE site:blog SET name = 'Blog';
                        CREATE site:forum SET name = 'Forum';
                        CREATE site:generator SET name = 'Generator';
                        CREATE site:marketplace SET name = 'Marketplace';
                        CREATE site:media_host SET name = 'Media Host';
                        CREATE site:news_publication SET name = 'News Publication';
                        CREATE site:reference SET name = 'Reference';
                        CREATE site:social_media_page SET name = 'Social Media Page';
                        CREATE site:social_network SET name = 'Social Network';

                        -- subcultures
                        CREATE subculture:album SET name = 'Album';
                        CREATE subculture:anime SET name = 'Anime';
                        CREATE subculture:blockchain SET name = 'Blockchain';
                        CREATE subculture:book SET name = 'Book';
                        CREATE subculture:cartoon SET name = 'Cartoon';
                        CREATE subculture:comic_book SET name = 'Comic Book';
                        CREATE subculture:company SET name = 'Company';
                        CREATE subculture:fauna SET name = 'Fauna';
                        CREATE subculture:fetish SET name = 'Fetish';
                        CREATE subculture:film SET name = 'Film';
                        CREATE subculture:manga SET name = 'Manga';
                        CREATE subculture:podcast SET name = 'Podcast';
                        CREATE subculture:product SET name = 'Product';
                        CREATE subculture:tv_show SET name = 'TV Show';
                        CREATE subculture:tabletop_games SET name = 'Tabletop Games';
                        CREATE subculture:theater SET name = 'Theater';
                        CREATE subculture:video_game SET name = 'Video Game';
                        CREATE subculture:web_series SET name = 'Web Series';
                        CREATE subculture:webcomic SET name = 'Webcomic';
                        -- added subcultures
                        CREATE subculture:alt_right SET name = 'Alt-Right';
                        CREATE subculture:alt_tech SET name = 'Alt-Tech';
                        CREATE subculture:anarchism SET name = 'Anarchism';

                        CREATE tag SET name = 'NSFW';

                        CREATE culture:e_acc SET name = 'e/acc', description = 'Effective Accelerationism', year = 2022, origin = 'twitter', region = 'United States', about = 'E/acc is an acronym for the phrase...';
                        CREATE person:nick_land SET name = 'Nick Land', year = 2005, origin = 'United Kingdom', about = 'Nick Land (born 17 January 1962) is an English philosopher...';
                        CREATE person:curtis_yarvin SET name = 'Curtis Yarvin', year = 2005, origin = 'United States', about = 'Curtis Guy Yarvin (born 1973), also known by his pen name Mencius Moldbug, is an American far-right blogger...';
                        CREATE person:mencius_moldbug SET name = 'Mencius Moldbug', year = 2005, origin = 'United States', about = 'Curtis Guy Yarvin (born 1973), also known by his pen name Mencius Moldbug, is an American far-right blogger...';
                        CREATE person:beff_jezos SET name = 'Beff Jezos', year = 2022, origin = 'United States', about = 'Beff Jezos is a Twitter alt for...';

                        CREATE event SET name = 'Union Solidarity Coalition Celebrity eBay Auction', year = 2023, origin = 'ebay', region = 'United States', about = 'lorem ipsum';
                        CREATE meme SET name = 'Zuck Bunker', year = 2023, origin = 'wired', about = 'Zuck Bunker refers to a compound...';
                        CREATE person SET name = 'Hulk Hogan', year = 2005, origin = 'United States', about = 'Hulk Hogan (born Terry Gene Bollea, August 11th, 1953)...';
                        CREATE site SET name = 'Polygon', year = 2012, origin = 'Vox Media', about = 'Polygon is a video game site...';
                        CREATE subculture SET name = 'Polygon', year = 2021, origin = 'StoneToss', about = 'StoneToss Flurk NFT is the name of a crypto art collection...';

                        RELATE person:beff_jezos->is_leader->culture:e_acc;
                        COMMIT TRANSACTION;
                    "#;
                    // RELATE category->culture;
                    // RELATE category->event;
                    // RELATE category->meme;
                    // RELATE category->person;
                    // RELATE category->site;
                    // RELATE category->subculture;

                    // RELATE tag->culture WHERE tag.name = 'NSFW' AND culture.name = 'e/acc';

                    // RELATE type->culture WHERE type.name IN ['Movement', 'Technology'] AND culture.name = 'e/acc';
                    // RELATE type->event WHERE type.name = 'Auction' AND event.name = 'Union Solidarity Coalition Celebrity eBay Auction';

                    // RELATE subculture->culture WHERE subculture.name = 'Neoreaction' AND culture.name = 'e/acc';
            
                    // RELATE culture->person WHERE culture.tags CONTAINS 'bunker' AND person.tags CONTAINS 'bunker';
                    let db = graphdb::open(our.package_id(), db)?; 
                    let result = match db.write(
                        init_script.to_string(),
                        None
                    ) {
                        Ok(result) => result,
                        Err(e) => {
                            println!("graphdb_test: db.query error: {:?}", e);
                            return Ok(());
                        }
                    };

                    println!("graphdb_test: init {:?}", result);

                    Response::new()
                        .body(serde_json::to_vec(&TestResponse::InitScriptRan).unwrap())
                        .send()
                        .unwrap();
                }
                //  /m our@graphdb_test:graphdb_test:template.os {"Open": {"db": "test_db"}}
                TestRequest::Open { ref db } => {
                    let db = graphdb::open(our.package_id(), db)?; 
                    
                    // Define the table
                    let _ = db.define(graphdb::DefineResourceType::Table {
                        name: "person".into(),
                    });

                    // Create a person
                    let result = match db.write(
                        "CREATE person SET name = $name, company = $company;".to_string(),
                        Some(serde_json::to_value(Person {
                            name: "John Doe",
                            company: "Acme",
                            
                        })?),
                    ) {
                        Ok(result) => result,
                        Err(e) => {
                            println!("graphdb_test: db.query error: {:?}", e);
                            return Ok(());
                        }
                    };

                    println!("\n graphdb_test: db.write {:?}", result);

                    let read_result = db.read("SELECT * FROM person;".into())?;

                    println!("\n graphdb_test: db.read {:?}", read_result);

                    Response::new()
                        .body(serde_json::to_vec(&TestResponse::DbOpened).unwrap())
                        .send()
                        .unwrap();
                }
                // /m our@graphdb_test:graphdb_test:template.os {"Read": {"db": "test_db", "query": "SELECT ->is_category FROM culture;"}}
                // /m our@graphdb_test:graphdb_test:template.os {"Read": {"db": "test_db", "query": "SELECT * FROM type;"}}
                TestRequest::Read { ref db, ref query } => {
                    let db = graphdb::open(our.package_id(), db)?;

                    let result = db.read(query.to_string())?;

                    println!("graphdb_test: db.read {}", result.to_string());

                    Response::new()
                        .body(serde_json::to_vec(&result).unwrap())
                        .send()
                        .unwrap();
                }
                // /m our@graphdb_test:graphdb_test:template.os {"Write": {"db": "test_db", "statement": "CREATE type:movement SET name = 'Movement';"}}
                TestRequest::Write { ref db, ref statement } => {
                    let db = graphdb::open(our.package_id(), db)?;

                    let result = db.write(statement.to_string(), None)?;
                    println!("graphdb_test: db.write {:?}", result);

                    Response::new()
                        .body(serde_json::to_vec(&result).unwrap())
                        .send()
                        .unwrap();
                }
            }
        }
    }
    Ok(())
}

call_init!(init);

fn init(our: Address) {
    println!("graphdb_test: begin");

    loop {
        match handle_message(&our) {
            Ok(()) => {}
            Err(e) => {
                println!("{:?}", e);
            }
        };
    }
}
