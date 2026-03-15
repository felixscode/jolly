use std::path::PathBuf;
use std::time::Instant;

fn model_path() -> PathBuf {
    dirs_next::data_dir()
        .unwrap_or_else(|| PathBuf::from("/home/dev/.local/share"))
        .join("com.jolly.desktop/models/jolly-model.gguf")
}

/// Integration test for local inference with a real model.
/// Run with: cargo test --test local_inference -- --nocapture
#[test]
fn test_local_inference_spell_check() {
    let path = model_path();
    if !path.exists() {
        eprintln!("SKIPPED: Model not found at {}", path.display());
        return;
    }

    let load_start = Instant::now();
    jolly_lib::inference::local::init_model(&path).expect("Failed to load model");
    eprintln!("\n=== Model loaded in {:.2}s ===\n", load_start.elapsed().as_secs_f64());

    // 100 sentences with typos
    let sentences = [
        "I recieved your messege yesterday.",
        "The wether was beautful today.",
        "Ther are meny pepole who dont undrestand grammer.",
        "She definately went to the libary after scool.",
        "He was very excitted about the upcomming event.",
        "The goverment annouced new policys today.",
        "We should seperate the diffrent categries.",
        "The resturant had excelent sevice.",
        "Its importent to maintane a helthy lifestyle.",
        "The enviroment is a crutial topic for discusion.",
        "They reccomend exercize for atleast thirty minuts.",
        "The professer gave a fasinating lecure.",
        "I beleive the calender is incorect.",
        "The commitee made a unanmous desicion.",
        "She accidently droped her favrite vase.",
        "The neigbors were extremly noisy last nite.",
        "He acheived a remarkble score on the examinaton.",
        "The occassion called for a speical celebraton.",
        "We experinced significnt delays during the jurney.",
        "The managment team adressed the isue promptly.",
        "Their knowlege of the subjct was impressve.",
        "The medcine had an immedite efect on the patint.",
        "She demostrated extrordinary skilz in the competion.",
        "The organizaton planed a succesful fundraser.",
        "He recieved a scholership for his acadmic achivments.",
        "The tecnology has advnced considrably in recnt years.",
        "We apprecate your patince during this dificult time.",
        "The buisness expanded its operatons to foriegn markets.",
        "She maintaned her composre despite the presure.",
        "The audiance was captivted by the performnce.",
        "He explined the proceedure in grate detail.",
        "The orignal document was misplased somewhere.",
        "They recgnized the importnce of educaton.",
        "The tempature droped significntly overnite.",
        "She perferred the tradtional approch to the problm.",
        "The infomation was completly inacurate.",
        "He volnteered to asist with the projct.",
        "The accomodations were quite comfortble.",
        "We witnesed an incredibal sunset yesterday.",
        "The corrispondence was delivred to the rong adress.",
        "She posesses remarkble inteligence and determanation.",
        "The gaurds patroled the premisis all nite.",
        "He has a tendancy to exagerate his storys.",
        "The vegtables were surprizingly fresh at the markt.",
        "They dicided to pospone the meting untill friday.",
        "The dissapointment was visable on her fase.",
        "She carriyed the grocerys to the vehical.",
        "The independant investigaton revealled new evidnce.",
        "He was concerened about the finacial situaton.",
        "The arguement escalted quickly between the neigbors.",
        "She recomended a wonderfull book to read.",
        "The occurence was completly unexpectd.",
        "He necesarily had to revize his orignal plan.",
        "The apparant cause of the problm was identifed.",
        "She was truely greatful for the oppertunity.",
        "The milennials have diffrent priorites than previus generatons.",
        "He persistantly worked on improveing his writting.",
        "The rythm of the musik was absolutly captivating.",
        "She acknowledgd the mistke and apologised sinceerly.",
        "The parliment debatted the contraversial legislaton.",
        "He garanteed the delivry would arive on tyme.",
        "The harasment polcy was updted last mounth.",
        "She struggled with prononciation of foriegn words.",
        "The labortory results confirmd the orignal hypothsis.",
        "He maintaned his innocense throughot the trial.",
        "The questionaire was distrbuted to all participnts.",
        "She ocassionally forgts her apointments.",
        "The beaurocracy made the proccess extremly slow.",
        "He aknowledged that the sitution was challanging.",
        "The vacume cleaner stoped working sudenly.",
        "She recieved a compliment on her pronounciation.",
        "The lisence was renewd without any dificulty.",
        "He misspeled several wrds in the documnt.",
        "The parliment passed the ammendment unanimosly.",
        "She excercised regulrly to stay in shpe.",
        "The brochure containd several gramatical erors.",
        "He beleived the prophcy would come tru.",
        "The guidence counseler ofered valueable advise.",
        "She noticied the discrpancy in the finacial records.",
        "The tommorow forecast predicts heavy rainfal.",
        "He reccommended a thourgh examinaton of the evidnce.",
        "The priviledge of attending was not taken lightely.",
        "She demostrated proficency in multple languges.",
        "The occurances were becming more frequant.",
        "He acheved his goales through persistnce and determinaton.",
        "The restarant recieved excelant reviews from critcs.",
        "She beleives strongly in the importnce of litteracy.",
        "The goverment oficials met to disscuss the crises.",
        "He inadvertntly revealed the suprise to evryone.",
        "The cemetary was locted on the outskrts of towne.",
        "She conciously decided to persue a diferent carreer.",
        "The calander had severl incorect dates listd.",
        "He was embarased by his mispronunciation.",
        "The comittee anounced the recipents of the awrd.",
        "She consistantly deliverd high qualty work.",
        "The foriegn delegaton arived ahead of schedle.",
        "He refered to the dictionry for the corect speling.",
        "The neccessary preprations were completd on tyme.",
        "She succesfully negotited a favorble agreement.",
        "The achievment was celebrated by the entier comunity.",
    ];

    let mut total_time = 0.0_f64;
    let mut successes = 0;
    let mut failures = 0;

    for (i, input) in sentences.iter().enumerate() {
        let start = Instant::now();
        let result = jolly_lib::inference::local::run_inference(input);
        let elapsed = start.elapsed().as_secs_f64();
        total_time += elapsed;

        match result {
            Ok(corrected) => {
                successes += 1;
                eprintln!(
                    "[{:3}/100] {:.2}s | \"{}\" -> \"{}\"",
                    i + 1,
                    elapsed,
                    input,
                    corrected
                );
            }
            Err(e) => {
                failures += 1;
                eprintln!("[{:3}/100] FAIL | \"{}\" -> Error: {}", i + 1, input, e);
            }
        }
    }

    eprintln!("\n=== Results ===");
    eprintln!("Total:    {:.2}s for {} sentences", total_time, sentences.len());
    eprintln!("Average:  {:.2}s per sentence", total_time / sentences.len() as f64);
    eprintln!("Success:  {}/{}", successes, sentences.len());
    eprintln!("Failures: {}", failures);

    assert_eq!(failures, 0, "{} sentences failed inference", failures);
}
