use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use jolly_lib::inference::local;
use jolly_lib::inference::registry::MODELS;

struct TestCase {
    id: usize,
    language: &'static str,
    category: &'static str,
    input: &'static str,
    expected: &'static str,
}

/// Read RSS (Resident Set Size) in MB from /proc/self/status.
fn rss_mb() -> f64 {
    fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<f64>().ok())
        })
        .map(|kb| kb / 1024.0)
        .unwrap_or(0.0)
}

/// Word-level similarity: proportion of expected words present in output.
/// Returns a score from 0.0 to 1.0.
fn word_similarity(expected: &str, output: &str) -> f64 {
    let expected_words: Vec<&str> = expected.split_whitespace().collect();
    if expected_words.is_empty() {
        return if output.trim().is_empty() { 1.0 } else { 0.0 };
    }
    let output_lower = output.to_lowercase();
    let matched = expected_words
        .iter()
        .filter(|w| output_lower.contains(&w.to_lowercase()))
        .count();
    matched as f64 / expected_words.len() as f64
}

fn test_cases() -> Vec<TestCase> {
    let mut id = 0;
    let mut next_id = || {
        id += 1;
        id
    };

    vec![
        // ── Short EN (12) ──────────────────────────────────────
        TestCase {
            id: next_id(), language: "en", category: "short",
            input: "I recieved your messege yesterday.",
            expected: "I received your message yesterday.",
        },
        TestCase {
            id: next_id(), language: "en", category: "short",
            input: "The wether was beautful today.",
            expected: "The weather was beautiful today.",
        },
        TestCase {
            id: next_id(), language: "en", category: "short",
            input: "She definately went to the libary after scool.",
            expected: "She definitely went to the library after school.",
        },
        TestCase {
            id: next_id(), language: "en", category: "short",
            input: "He was very excitted about the upcomming event.",
            expected: "He was very excited about the upcoming event.",
        },
        TestCase {
            id: next_id(), language: "en", category: "short",
            input: "The goverment annouced new policys today.",
            expected: "The government announced new policies today.",
        },
        TestCase {
            id: next_id(), language: "en", category: "short",
            input: "Its importent to maintane a helthy lifestyle.",
            expected: "It's important to maintain a healthy lifestyle.",
        },
        TestCase {
            id: next_id(), language: "en", category: "short",
            input: "The resturant had excelent sevice.",
            expected: "The restaurant had excellent service.",
        },
        TestCase {
            id: next_id(), language: "en", category: "short",
            input: "We should seperate the diffrent categries.",
            expected: "We should separate the different categories.",
        },
        TestCase {
            id: next_id(), language: "en", category: "short",
            input: "The commitee made a unanmous desicion.",
            expected: "The committee made a unanimous decision.",
        },
        TestCase {
            id: next_id(), language: "en", category: "short",
            input: "She accidently droped her favrite vase.",
            expected: "She accidentally dropped her favorite vase.",
        },
        TestCase {
            id: next_id(), language: "en", category: "short",
            input: "The professer gave a fasinating lecure.",
            expected: "The professor gave a fascinating lecture.",
        },
        TestCase {
            id: next_id(), language: "en", category: "short",
            input: "I beleive the calender is incorect.",
            expected: "I believe the calendar is incorrect.",
        },

        // ── Short DE (8) ───────────────────────────────────────
        TestCase {
            id: next_id(), language: "de", category: "short",
            input: "Ich habe gestern deine Nachich erhalten.",
            expected: "Ich habe gestern deine Nachricht erhalten.",
        },
        TestCase {
            id: next_id(), language: "de", category: "short",
            input: "Das Weter war heute wunderschon.",
            expected: "Das Wetter war heute wunderschön.",
        },
        TestCase {
            id: next_id(), language: "de", category: "short",
            input: "Wir solten uns morgen trefen.",
            expected: "Wir sollten uns morgen treffen.",
        },
        TestCase {
            id: next_id(), language: "de", category: "short",
            input: "Die Besprechung wurde auf nechste Woche verschoben.",
            expected: "Die Besprechung wurde auf nächste Woche verschoben.",
        },
        TestCase {
            id: next_id(), language: "de", category: "short",
            input: "Er hat die Aufgabe volstandig erledigt.",
            expected: "Er hat die Aufgabe vollständig erledigt.",
        },
        TestCase {
            id: next_id(), language: "de", category: "short",
            input: "Die Regirung hat neue Massnahmen beschlosen.",
            expected: "Die Regierung hat neue Maßnahmen beschlossen.",
        },
        TestCase {
            id: next_id(), language: "de", category: "short",
            input: "Bitte schiken Sie mir die Unterlangen.",
            expected: "Bitte schicken Sie mir die Unterlagen.",
        },
        TestCase {
            id: next_id(), language: "de", category: "short",
            input: "Das Ergbnis war uberraschend positiv.",
            expected: "Das Ergebnis war überraschend positiv.",
        },

        // ── Medium EN (8) ──────────────────────────────────────
        TestCase {
            id: next_id(), language: "en", category: "medium",
            input: "I wanted to let you know that the meting has been rescheduld. Please make sure to update your calender acordingly.",
            expected: "I wanted to let you know that the meeting has been rescheduled. Please make sure to update your calendar accordingly.",
        },
        TestCase {
            id: next_id(), language: "en", category: "medium",
            input: "The new sofware update has been relased. All employes should instal it before the end of the week.",
            expected: "The new software update has been released. All employees should install it before the end of the week.",
        },
        TestCase {
            id: next_id(), language: "en", category: "medium",
            input: "We are planing to expend our operatons to new markets. The stratigic review is schedled for next mounth.",
            expected: "We are planning to expand our operations to new markets. The strategic review is scheduled for next month.",
        },
        TestCase {
            id: next_id(), language: "en", category: "medium",
            input: "The projct deliverables are due by Firday. Please ensure all documention is completd and submited on time.",
            expected: "The project deliverables are due by Friday. Please ensure all documentation is completed and submitted on time.",
        },
        TestCase {
            id: next_id(), language: "en", category: "medium",
            input: "Thank you for your patiance during this transistion period. We apreciate your continiued support and understading.",
            expected: "Thank you for your patience during this transition period. We appreciate your continued support and understanding.",
        },
        TestCase {
            id: next_id(), language: "en", category: "medium",
            input: "The reserch team has publishd there findings. The resuts are very promissing and could lead to signifcant breakthroughs.",
            expected: "The research team has published their findings. The results are very promising and could lead to significant breakthroughs.",
        },
        TestCase {
            id: next_id(), language: "en", category: "medium",
            input: "Our custmer satisfation scores have improoved significently. This is larglely due to the new traning programm we implmented.",
            expected: "Our customer satisfaction scores have improved significantly. This is largely due to the new training program we implemented.",
        },
        TestCase {
            id: next_id(), language: "en", category: "medium",
            input: "The budjet for next quater has been aproved. We can now procede with hiring aditional staff for the departement.",
            expected: "The budget for next quarter has been approved. We can now proceed with hiring additional staff for the department.",
        },

        // ── Medium DE (5) ──────────────────────────────────────
        TestCase {
            id: next_id(), language: "de", category: "medium",
            input: "Ich wollte Ihnen mitteilen, dass die Besprechng verschoben wurde. Bitte aktualisiren Sie Ihren Kalender entsprchend.",
            expected: "Ich wollte Ihnen mitteilen, dass die Besprechung verschoben wurde. Bitte aktualisieren Sie Ihren Kalender entsprechend.",
        },
        TestCase {
            id: next_id(), language: "de", category: "medium",
            input: "Das neue Softwar-Update wurde veröfentlicht. Alle Mitarbeiter solten es vor Ende der Woche instalieren.",
            expected: "Das neue Software-Update wurde veröffentlicht. Alle Mitarbeiter sollten es vor Ende der Woche installieren.",
        },
        TestCase {
            id: next_id(), language: "de", category: "medium",
            input: "Wir planen unsere Geschäfstätigkeit auf neue Märkte auszuweiten. Die strategische Überprüfung ist für nächsten Monat geplannt.",
            expected: "Wir planen unsere Geschäftstätigkeit auf neue Märkte auszuweiten. Die strategische Überprüfung ist für nächsten Monat geplant.",
        },
        TestCase {
            id: next_id(), language: "de", category: "medium",
            input: "Vielen Dank für Ihre Gedult während dieser Übergangsphase. Wir schätzen Ihre fortwärende Unterstüzung und Ihr Verständniss.",
            expected: "Vielen Dank für Ihre Geduld während dieser Übergangsphase. Wir schätzen Ihre fortwährende Unterstützung und Ihr Verständnis.",
        },
        TestCase {
            id: next_id(), language: "de", category: "medium",
            input: "Das Forschungsteam hat seine Ergebnise veröfentlicht. Die Resultate sind sehr vielversprechnd und könten zu bedeutenden Durchbrüchen füren.",
            expected: "Das Forschungsteam hat seine Ergebnisse veröffentlicht. Die Resultate sind sehr vielversprechend und könnten zu bedeutenden Durchbrüchen führen.",
        },

        // ── Long EN (4) ────────────────────────────────────────
        TestCase {
            id: next_id(), language: "en", category: "long",
            input: "The anual report has been finalized and is reddy for distribusion. It includes a comprehensve overview of our finacial performence, key achivements, and strategec goals for the upcomming year. All departement heads should reveiw the relevent sections and provide there feedback by the end of next week.",
            expected: "The annual report has been finalized and is ready for distribution. It includes a comprehensive overview of our financial performance, key achievements, and strategic goals for the upcoming year. All department heads should review the relevant sections and provide their feedback by the end of next week.",
        },
        TestCase {
            id: next_id(), language: "en", category: "long",
            input: "We are pleased to anounce that our compeny has been reconized as one of the top emploiers in the region. This acheivement reflecs our comitment to creating a positive work enviroment and investing in our employes profesional developement. We would like to thank evreyone for there hard work and dedicaton.",
            expected: "We are pleased to announce that our company has been recognized as one of the top employers in the region. This achievement reflects our commitment to creating a positive work environment and investing in our employees' professional development. We would like to thank everyone for their hard work and dedication.",
        },
        TestCase {
            id: next_id(), language: "en", category: "long",
            input: "Following our disscusion last week, I wanted to summerize the key action itmes. First, the marketing team will prepair a revised campain proposel by Wensday. Second, the enginering departement will conduct a feasability study on the new product lien. Third, finace will provide an updated budjet forcast for the remainding quater.",
            expected: "Following our discussion last week, I wanted to summarize the key action items. First, the marketing team will prepare a revised campaign proposal by Wednesday. Second, the engineering department will conduct a feasibility study on the new product line. Third, finance will provide an updated budget forecast for the remaining quarter.",
        },
        TestCase {
            id: next_id(), language: "en", category: "long",
            input: "The custmer feedback survay results are in, and they paint a very encuraging picture. Overal satisfation has incresed by fifteen percent compaired to last quater. The areas that recieved the highist marks were custmer suport responsivness and product qualty. However, there are still oppertunities for improvment in our delivry timelines and onlien ordering expirience.",
            expected: "The customer feedback survey results are in, and they paint a very encouraging picture. Overall satisfaction has increased by fifteen percent compared to last quarter. The areas that received the highest marks were customer support responsiveness and product quality. However, there are still opportunities for improvement in our delivery timelines and online ordering experience.",
        },

        // ── Long DE (3) ────────────────────────────────────────
        TestCase {
            id: next_id(), language: "de", category: "long",
            input: "Der Jahresbericht wurde fertiggestelt und ist bereit zur Verteilung. Er enthält einen umfasenden Überblick über unsere finanziele Leistung, wichtige Errungenschaften und strategische Ziele für das komende Jahr. Alle Abteilungsleiter solten die relevanten Abschnite überprüfen und bis Ende nächster Woche ihr Fedback geben.",
            expected: "Der Jahresbericht wurde fertiggestellt und ist bereit zur Verteilung. Er enthält einen umfassenden Überblick über unsere finanzielle Leistung, wichtige Errungenschaften und strategische Ziele für das kommende Jahr. Alle Abteilungsleiter sollten die relevanten Abschnitte überprüfen und bis Ende nächster Woche ihr Feedback geben.",
        },
        TestCase {
            id: next_id(), language: "de", category: "long",
            input: "Wir freuen uns mitzutelen, dass unser Unternehmen als einer der besten Arbeitgeber der Region ausgezeichent wurde. Diese Auszeichung spiegelt unser Engagment für die Schafung eines positiven Arbeitsumfeldes und die Investition in die berufliche Entwickelung unserer Mitarbeiter wider. Wir möchten allen für ihre harte Arbeit und Hingabe dancken.",
            expected: "Wir freuen uns mitzuteilen, dass unser Unternehmen als einer der besten Arbeitgeber der Region ausgezeichnet wurde. Diese Auszeichnung spiegelt unser Engagement für die Schaffung eines positiven Arbeitsumfeldes und die Investition in die berufliche Entwicklung unserer Mitarbeiter wider. Wir möchten allen für ihre harte Arbeit und Hingabe danken.",
        },
        TestCase {
            id: next_id(), language: "de", category: "long",
            input: "Nach unserer Diskusion letzte Woche möchte ich die wichtigsten Aktionspunkte zusammenfasen. Erstens wird das Marketingteam bis Mitwoch einen überarbeiteten Kampagnenvorschlag erstelen. Zweitens wird die Ingenieursabteilung eine Machbarkeitsstudie für die neue Produktlinie durchfüren. Drittens wird die Finanzabteilung eine aktualiesierte Budgetprognose für das verbleibende Quartal bereitstelen.",
            expected: "Nach unserer Diskussion letzte Woche möchte ich die wichtigsten Aktionspunkte zusammenfassen. Erstens wird das Marketingteam bis Mittwoch einen überarbeiteten Kampagnenvorschlag erstellen. Zweitens wird die Ingenieursabteilung eine Machbarkeitsstudie für die neue Produktlinie durchführen. Drittens wird die Finanzabteilung eine aktualisierte Budgetprognose für das verbleibende Quartal bereitstellen.",
        },

        // ── Full Email EN (5) ──────────────────────────────────
        TestCase {
            id: next_id(), language: "en", category: "email",
            input: "Hi Sarah,\n\nI hope this email findz you well. I wanted to follow up on our conversaton from last week regardng the new project timelien.\n\nAfter revieing the requiremants, I beleive we can meet the orignal deadlien if we allocate addtional resurces to the developement team. Could we scheduel a quick call tommorow to discus this furthur?\n\nBest regards,\nTom",
            expected: "Hi Sarah,\n\nI hope this email finds you well. I wanted to follow up on our conversation from last week regarding the new project timeline.\n\nAfter reviewing the requirements, I believe we can meet the original deadline if we allocate additional resources to the development team. Could we schedule a quick call tomorrow to discuss this further?\n\nBest regards,\nTom",
        },
        TestCase {
            id: next_id(), language: "en", category: "email",
            input: "Dear Mr. Johnson,\n\nThank you for your intrest in our servises. I am writting to confrim your apointment on Thurdsay, March 20th at 2:00 PM.\n\nPlease bring the folowing documets:\n- A valid photo ID\n- Your insurence card\n- Any relevent medical recods\n\nIf you need to rescheduel, please contac our ofice at least 24 hours in advanse.\n\nSincerely,\nDr. Emily Chen\nCity Medical Center",
            expected: "Dear Mr. Johnson,\n\nThank you for your interest in our services. I am writing to confirm your appointment on Thursday, March 20th at 2:00 PM.\n\nPlease bring the following documents:\n- A valid photo ID\n- Your insurance card\n- Any relevant medical records\n\nIf you need to reschedule, please contact our office at least 24 hours in advance.\n\nSincerely,\nDr. Emily Chen\nCity Medical Center",
        },
        TestCase {
            id: next_id(), language: "en", category: "email",
            input: "Hey team,\n\nQuick update on the Q2 planing:\n\n1. The markting budjet has been aprooved\n2. We need to finlize the hiring plan by Firday\n3. The product roadmapp review is next Teusday\n\nLet me know if you have any questons or conserns.\n\nCheers,\nAlex",
            expected: "Hey team,\n\nQuick update on the Q2 planning:\n\n1. The marketing budget has been approved\n2. We need to finalize the hiring plan by Friday\n3. The product roadmap review is next Tuesday\n\nLet me know if you have any questions or concerns.\n\nCheers,\nAlex",
        },
        TestCase {
            id: next_id(), language: "en", category: "email",
            input: "Dear Hiring Comittee,\n\nI am writting to express my intrest in the Senior Sofware Enginere postion advertized on your websight. With over ten years of experiance in full-stack developement, I am confidant that my skills and backround make me an excelent candidat for this role.\n\nI have atached my resume and cover leter for your reveiw. I look forwrd to the opertunity to discus how I can contribut to your team.\n\nThank you for your considration.\n\nBest regards,\nMichael Torres",
            expected: "Dear Hiring Committee,\n\nI am writing to express my interest in the Senior Software Engineer position advertised on your website. With over ten years of experience in full-stack development, I am confident that my skills and background make me an excellent candidate for this role.\n\nI have attached my resume and cover letter for your review. I look forward to the opportunity to discuss how I can contribute to your team.\n\nThank you for your consideration.\n\nBest regards,\nMichael Torres",
        },
        TestCase {
            id: next_id(), language: "en", category: "email",
            input: "Subject: Urgant - Server Maintanence Schedled\n\nDear all,\n\nPlease be advized that we will be preforming schedled maintanence on our producion servers this Saterday from 10:00 PM to 4:00 AM.\n\nDuring this peroid, the folowing servises will be unavailble:\n- Email\n- Cloud storiage\n- Internal dashbords\n\nWe apologise for any inconveneince and apreciate your understandng.\n\nIT Departement",
            expected: "Subject: Urgent - Server Maintenance Scheduled\n\nDear all,\n\nPlease be advised that we will be performing scheduled maintenance on our production servers this Saturday from 10:00 PM to 4:00 AM.\n\nDuring this period, the following services will be unavailable:\n- Email\n- Cloud storage\n- Internal dashboards\n\nWe apologize for any inconvenience and appreciate your understanding.\n\nIT Department",
        },

        // ── Full Email DE (5) ──────────────────────────────────
        TestCase {
            id: next_id(), language: "de", category: "email",
            input: "Hallo Frau Müller,\n\nich hofe, diese E-Mail ereicht Sie gut. Ich wollte mich bezüglich unseres Gesprächs von letzter Woche zum neuen Projektzeitplan melden.\n\nNach Durchsicht der Anforderugen bin ich der Meinung, dass wir den ursprünglichen Termin einhalten könen, wenn wir dem Entwicklungsteam zusätzliche Ressorcen zuweisen. Könnten wir morgen einen kurzen Anruf vereinbahren, um dies weiter zu bespreschen?\n\nMit freundlichen Grüßen,\nThomas Schmidt",
            expected: "Hallo Frau Müller,\n\nich hoffe, diese E-Mail erreicht Sie gut. Ich wollte mich bezüglich unseres Gesprächs von letzter Woche zum neuen Projektzeitplan melden.\n\nNach Durchsicht der Anforderungen bin ich der Meinung, dass wir den ursprünglichen Termin einhalten können, wenn wir dem Entwicklungsteam zusätzliche Ressourcen zuweisen. Könnten wir morgen einen kurzen Anruf vereinbaren, um dies weiter zu besprechen?\n\nMit freundlichen Grüßen,\nThomas Schmidt",
        },
        TestCase {
            id: next_id(), language: "de", category: "email",
            input: "Sehr geerter Herr Weber,\n\nvielen Dank für Ihr Interesse an unseren Dienstleistunegn. Ich schreibe Ihnen, um Ihren Termin am Donerstag, den 20. März um 14:00 Uhr zu bestatigen.\n\nBitte bringen Sie folgene Dokumente mit:\n- Einen gültigen Lichtbildausweis\n- Ihre Versicherungskarte\n- Alle relevanten medizinischen Unterlangen\n\nWenn Sie umbuchen müsen, kontaktieren Sie bitte unser Büro mindestons 24 Stunden im Vorraus.\n\nMit freundlichen Grüßen,\nDr. Anna Fischer\nStadtklinik Berlin",
            expected: "Sehr geehrter Herr Weber,\n\nvielen Dank für Ihr Interesse an unseren Dienstleistungen. Ich schreibe Ihnen, um Ihren Termin am Donnerstag, den 20. März um 14:00 Uhr zu bestätigen.\n\nBitte bringen Sie folgende Dokumente mit:\n- Einen gültigen Lichtbildausweis\n- Ihre Versicherungskarte\n- Alle relevanten medizinischen Unterlagen\n\nWenn Sie umbuchen müssen, kontaktieren Sie bitte unser Büro mindestens 24 Stunden im Voraus.\n\nMit freundlichen Grüßen,\nDr. Anna Fischer\nStadtklinik Berlin",
        },
        TestCase {
            id: next_id(), language: "de", category: "email",
            input: "Hallo Team,\n\nkurzes Update zur Q2-Plannung:\n\n1. Das Marketingbudget wurde genehimgt\n2. Wir müssen den Einstellungsplan bis Frietag finalisiren\n3. Die Produkt-Roadmap-Überprüfung ist nächsten Dienstag\n\nLasst mich wissen, wenn ihr Fragen oder Bedenken habbt.\n\nViele Grüße,\nAlexander",
            expected: "Hallo Team,\n\nkurzes Update zur Q2-Planung:\n\n1. Das Marketingbudget wurde genehmigt\n2. Wir müssen den Einstellungsplan bis Freitag finalisieren\n3. Die Produkt-Roadmap-Überprüfung ist nächsten Dienstag\n\nLasst mich wissen, wenn ihr Fragen oder Bedenken habt.\n\nViele Grüße,\nAlexander",
        },
        TestCase {
            id: next_id(), language: "de", category: "email",
            input: "Sehr geehrtes Einstellungskomitee,\n\nmit großem Interesse bewerbe ich mich auf die ausgeschriebne Stelle als Senior Software Engenieur auf Ihrer Webseite. Mit über zehn Jaren Erfahrung in der Full-Stack-Entwickelung bin ich überzeugt, dass meine Fähigkeiten und mein Hintergrund mich zu einem ausgezeichnetten Kandidaten für diese Rolle machen.\n\nIch habe meinen Lebenslauf und mein Anschreiben zur Durchsicht beigefügt. Ich freue mich auf die Möglichkeit zu bespreschen, wie ich zu Ihrem Team beitragen kan.\n\nVielen Dank für Ihre Berücksichtigung.\n\nMit freundlichen Grüßen,\nMichael Wagner",
            expected: "Sehr geehrtes Einstellungskomitee,\n\nmit großem Interesse bewerbe ich mich auf die ausgeschriebene Stelle als Senior Software Ingenieur auf Ihrer Webseite. Mit über zehn Jahren Erfahrung in der Full-Stack-Entwicklung bin ich überzeugt, dass meine Fähigkeiten und mein Hintergrund mich zu einem ausgezeichneten Kandidaten für diese Rolle machen.\n\nIch habe meinen Lebenslauf und mein Anschreiben zur Durchsicht beigefügt. Ich freue mich auf die Möglichkeit zu besprechen, wie ich zu Ihrem Team beitragen kann.\n\nVielen Dank für Ihre Berücksichtigung.\n\nMit freundlichen Grüßen,\nMichael Wagner",
        },
        TestCase {
            id: next_id(), language: "de", category: "email",
            input: "Betreff: Dringent - Geplannte Serverwartung\n\nLiebe Kolleginnen und Kollegen,\n\nbitte beachten Sie, dass wir diesen Samstag von 22:00 bis 4:00 Uhr eine geplannte Wartung an unseren Produktionsservern durchfüren werden.\n\nWährend dieses Zeitraums werden folgene Dienste nicht verfügbar sein:\n- E-Mail\n- Cloud-Speicher\n- Interne Dashbords\n\nWir entschuldigen uns für etwaige Unanemlichkeiten und danken für Ihr Verständniss.\n\nIT-Abteilung",
            expected: "Betreff: Dringend - Geplante Serverwartung\n\nLiebe Kolleginnen und Kollegen,\n\nbitte beachten Sie, dass wir diesen Samstag von 22:00 bis 4:00 Uhr eine geplante Wartung an unseren Produktionsservern durchführen werden.\n\nWährend dieses Zeitraums werden folgende Dienste nicht verfügbar sein:\n- E-Mail\n- Cloud-Speicher\n- Interne Dashboards\n\nWir entschuldigen uns für etwaige Unannehmlichkeiten und danken für Ihr Verständnis.\n\nIT-Abteilung",
        },
    ]
}

fn escape_csv(s: &str) -> String {
    // CSV: wrap in quotes, double any existing quotes
    let escaped = s.replace('"', "\"\"");
    format!("\"{}\"", escaped)
}

fn main() {
    let cases = test_cases();
    eprintln!("Loaded {} test cases", cases.len());

    // Find models directory
    let models_dir = dirs_next::data_dir()
        .unwrap_or_else(|| PathBuf::from("/home/dev/.local/share"))
        .join("com.jolly.desktop/models");

    if !models_dir.exists() {
        eprintln!("Models directory not found: {}", models_dir.display());
        eprintln!("Download models through the Jolly app first.");
        std::process::exit(1);
    }

    // Find downloaded models
    let downloaded: Vec<_> = MODELS
        .iter()
        .filter(|m| models_dir.join(m.file_name).exists())
        .collect();

    if downloaded.is_empty() {
        eprintln!("No downloaded models found in {}", models_dir.display());
        eprintln!("Available models:");
        for m in MODELS {
            eprintln!("  - {} ({})", m.name, m.file_name);
        }
        std::process::exit(1);
    }

    eprintln!("Found {} downloaded models:", downloaded.len());
    for m in &downloaded {
        eprintln!("  - {} ({})", m.name, m.file_name);
    }

    // Open CSV output
    let csv_path = PathBuf::from("benchmark_results.csv");
    let mut csv = fs::File::create(&csv_path).expect("Failed to create CSV file");
    writeln!(
        csv,
        "test_id,language,category,model_id,model_name,exact_match,similarity,time_ms,rss_mb,input,expected,output"
    )
    .unwrap();

    let rss_before_models = rss_mb();
    eprintln!("Baseline RSS: {:.0} MB", rss_before_models);

    // Run benchmark for each model
    for model in &downloaded {
        let model_path = models_dir.join(model.file_name);
        eprintln!("\n=== Loading model: {} ===", model.name);

        let load_start = Instant::now();
        if let Err(e) = local::init_model(&model_path) {
            eprintln!("FAILED to load {}: {}", model.name, e);
            continue;
        }
        let rss_after_load = rss_mb();
        eprintln!(
            "Model loaded in {:.1}s | RSS: {:.0} MB (+{:.0} MB)",
            load_start.elapsed().as_secs_f64(),
            rss_after_load,
            rss_after_load - rss_before_models,
        );

        for case in &cases {
            let start = Instant::now();
            let result = local::run_inference(case.input);
            let elapsed_ms = start.elapsed().as_millis();
            let current_rss = rss_mb();

            match result {
                Ok(output) => {
                    let trimmed = output.trim();
                    let exact_match = trimmed == case.expected.trim();
                    let similarity = word_similarity(case.expected, trimmed);
                    eprintln!(
                        "  [{:2}] {:2} {:6} {:>5}ms sim={:.2} {} | {}",
                        case.id,
                        case.language,
                        case.category,
                        elapsed_ms,
                        similarity,
                        if exact_match { "PASS" } else { "FAIL" },
                        &output.chars().take(60).collect::<String>(),
                    );
                    writeln!(
                        csv,
                        "{},{},{},{},{},{},{:.2},{},{:.0},{},{},{}",
                        case.id,
                        case.language,
                        case.category,
                        model.id,
                        escape_csv(model.name),
                        exact_match,
                        similarity,
                        elapsed_ms,
                        current_rss,
                        escape_csv(case.input),
                        escape_csv(case.expected),
                        escape_csv(&output),
                    )
                    .unwrap();
                }
                Err(e) => {
                    eprintln!(
                        "  [{:2}] {:2} {:6} ERROR: {}",
                        case.id, case.language, case.category, e
                    );
                    writeln!(
                        csv,
                        "{},{},{},{},{},false,0.00,{},{:.0},{},{},{}",
                        case.id,
                        case.language,
                        case.category,
                        model.id,
                        escape_csv(model.name),
                        elapsed_ms,
                        current_rss,
                        escape_csv(case.input),
                        escape_csv(case.expected),
                        escape_csv(&format!("ERROR: {}", e)),
                    )
                    .unwrap();
                }
            }
        }

        // Unload model before loading next one
        local::unload_model();
        eprintln!("Model unloaded | RSS: {:.0} MB", rss_mb());
    }

    csv.flush().unwrap();
    eprintln!("\n=== Results written to {} ===", csv_path.display());
}
