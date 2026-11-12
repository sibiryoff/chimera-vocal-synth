//! Russian text → phonemes: stress (marks, ё, the dictionary, the fallbacks), the spelling rules,
//! reduction in speech and song, voicing across words, and sung lyrics in plain Cyrillic.

use chimera::phonetics::english::EnglishG2P;
use chimera::phonetics::g2p::G2P;
use chimera::phonetics::russian::RussianG2P;
use chimera::phonetics::syllables::lyrics_to_syllables;
use std::io::Read;
use std::sync::OnceLock;

fn g2p() -> &'static G2P {
    static G: OnceLock<G2P> = OnceLock::new();
    G.get_or_init(|| {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../public/data/");
        let gunzip = |f: &str| {
            let gz = std::fs::read(format!("{root}{f}")).unwrap();
            let mut s = String::new();
            flate2::read::GzDecoder::new(&gz[..]).read_to_string(&mut s).unwrap();
            s
        };
        let mut g = G2P::new();
        g.en.load_dict(&gunzip("cmudict.txt.gz"));
        g.ru.load_dict(&gunzip("ru-stress.txt.gz"));
        g
    })
}

/// a phrase's segments as "ˈ"-marked IPA, words split by spaces
fn said(text: &str) -> String {
    g2p()
        .to_phrases(text)
        .iter()
        .map(|p| p.segments.iter().map(|s| format!("{}{}{}", if s.stress == Some(1) { "ˈ" } else { "" }, s.ipa, if s.word_end == Some(true) { " " } else { "" })).collect::<String>().trim().to_string())
        .collect::<Vec<_>>()
        .join(" | ")
}

fn sung(word: &str) -> String {
    g2p().word(word, false).iter().map(|s| format!("{}{}", if s.stress == Some(1) { "ˈ" } else { "" }, s.ipa)).collect()
}

#[test]
fn words_in_speech() {
    for (text, ipa) in [
        // stress from the dictionary, akanye and ikanye
        ("молоко", "mɐɫɐkˈo"),
        ("хорошо", "xɐrɐʂˈo"),
        ("язык", "jɪzˈɨk"),
        ("тебя", "tʲɪbʲˈa"),
        // ё restored by the dictionary (the stress source writes none), or written
        ("еще", "jɪɕˈo"),
        ("береза", "bʲɪrʲˈozɐ"),
        ("всё", "fsʲˈo"),
        // spelling exceptions
        ("что", "ʂtˈo"),
        ("его", "jɪvˈo"),
        ("красного", "krˈasnɐvɐ"),
        ("много", "mnˈogɐ"),
        ("сегодня", "sʲɪvˈodnʲɐ"),
        ("учиться", "ʊtɕˈitsɐ"),
        ("счастье", "ɕˈastʲjɪ"),
        ("солнце", "sˈontsɨ"),
        ("лестница", "lʲˈesnʲɪtsɐ"),
        ("легко", "lʲɪxkˈo"),
        // final devoicing, voicing assimilation
        ("лёд", "lʲˈot"),
        ("дуб", "dˈup"),
        ("сделать", "zdʲˈeɫɐtʲ"),
        ("вчера", "ftɕɪrˈa"),
        // ж ш ц hard, ч щ soft, и after them
        ("жена", "ʐɨnˈa"),
        ("шёл", "ʂˈoɫ"),
        // (no assimilative softness: [ʐɨznʲ], not the older [ʐɨzʲnʲ]; both are standard today)
        ("жизнь", "ʐˈɨznʲ"),
        ("часы", "tɕɪsˈɨ"),
        ("щука", "ɕˈukɐ"),
        // [j]: я е ё ю after a vowel, ь, ъ
        ("семья", "sʲɪmʲjˈa"),
        ("соловьи", "sɐɫɐvʲjˈi"),
        ("объём", "ɐbjˈom"),
        ("новая", "nˈovɐjɐ"),
        // hard л is [ɫ], soft [lʲ]
        ("мыла", "mˈɨɫɐ"),
        ("люблю", "lʲʊblʲˈu"),
    ] {
        assert_eq!(said(text), ipa, "{text}");
    }
}

#[test]
fn words_across_a_phrase() {
    // vowelless prepositions join the next word: hard before и ([ɨ]), voicing follows it
    assert_eq!(said("в избе"), "vɨzbʲˈe");
    assert_eq!(said("в лесу"), "vlʲɪsˈu");
    assert_eq!(said("к дому"), "gdˈomʊ");
    assert_eq!(said("с ёлки"), "sjˈoɫkʲɪ");
    // final devoicing, then the next word's voiced obstruent voices it back
    assert_eq!(said("сад был"), "sˈad bˈɨɫ");
    // the conjunction и is a full vowel of its own (reduced, it merged into its neighbours)
    assert_eq!(said("сад и"), "sˈat i");
    // clitics are unstressed in speech; a clitic compound has one stress
    assert_eq!(said("на столе"), "nɐ stɐlʲˈe");
    assert_eq!(said("кто-то"), "ktˈotɐ");
    // punctuation splits phrases, as in English
    assert_eq!(said("чижик-пыжик, где ты был?"), "tɕˈiʐɨkpˈɨʐɨk | gdʲˈe tˈɨ bˈɨɫ");
    let p = g2p().to_phrases("Где ты был?");
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].intonation, chimera::phonetics::Intonation::Question);
    assert_eq!(p[0].words, ["где", "ты", "был"]);
    // a Latin word in Russian text is read as English, with no English allophones
    assert!(said("мой iPhone").starts_with("mˈoj "));
}

#[test]
fn stress_from_the_text_and_without_a_dictionary() {
    let ru = RussianG2P::new();
    let s = |w: &str| ru.word(w, true).iter().map(|s| format!("{}{}", if s.stress == Some(1) { "ˈ" } else { "" }, s.ipa)).collect::<String>();
    // "+" or a combining acute after the stressed vowel
    assert_eq!(s("моло+ко"), "mɐɫˈokɐ");
    assert_eq!(s("за́мок"), "zˈamɐk");
    // ё is stressed
    assert_eq!(s("ёлка"), "jˈoɫkɐ");
    // one vowel: stressed; otherwise the penultimate, until the dictionary is loaded
    assert_eq!(s("дом"), "dˈom");
    assert_eq!(s("молоко"), "mɐɫˈokɐ");
    // the marks win over the dictionary
    assert_eq!(said("за́мок"), "zˈamɐk");
    assert_eq!(said("замо́к"), "zɐmˈok");
}

#[test]
fn sung_vowels_keep_their_quality() {
    // sung, only akanye stays (о → [a]); е я и keep theirs
    assert_eq!(sung("молоко"), "maɫakˈo");
    assert_eq!(sung("тебя"), "tʲebʲˈa");
    assert_eq!(sung("родилась"), "radʲiɫˈasʲ");
    assert_eq!(sung("на"), "nˈa");
}

#[test]
fn lyrics_in_plain_cyrillic() {
    let syl = lyrics_to_syllables("В лесу родилась ёлочка, в лесу она росла.", g2p());
    let texts: Vec<&str> = syl.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(texts, ["В ле", "су", "ро", "ди", "лась", "ё", "ло", "чка", "в ле", "су", "о", "на", "ро", "сла"]);
    let stressed: Vec<&str> = syl.iter().filter(|s| s.nucleus.stress == Some(1)).map(|s| s.text.as_str()).collect();
    assert_eq!(stressed, ["су", "лась", "ё", "су", "на", "сла"]);
    assert!(syl[7].phrase_end && syl[13].phrase_end);
    // a hyphenated pair of words is two words; sonorants close their syllable (фон-тан-ке)
    let syl = lyrics_to_syllables("Чижик-пыжик, где ты был? На Фонтанке водку пил.", g2p());
    let texts: Vec<&str> = syl.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(texts, ["Чи", "жик", "пы", "жик", "где", "ты", "был", "На", "Фон", "тан", "ке", "во", "дку", "пил"]);
    let onset = |k: usize| syl[k].onset.iter().map(|s| s.ipa.as_str()).collect::<String>();
    let coda = |k: usize| syl[k].coda.iter().map(|s| s.ipa.as_str()).collect::<String>();
    assert_eq!((coda(8), onset(9), coda(9), onset(10)), ("n".into(), "t".into(), "n".into(), "kʲ".into()));
    assert!(syl[1].word_end && syl[3].word_end);
    // ы after a hard consonant, and the sung "на" with a full [a]
    assert_eq!(syl[5].nucleus.ipa, "ɨ");
    assert_eq!(syl[7].nucleus.ipa, "a");
    // melisma marks and bracketed IPA still mix in
    let syl = lyrics_to_syllables("люблю ~ [tʲɪˈbʲa]", g2p());
    assert_eq!(syl.len(), 5);
    assert_eq!(syl[2].melisma, Some(true));
}

#[test]
fn english_text_is_read_as_before() {
    // the app's G2P sends text with no Cyrillic to the English G2P unchanged
    let en = &g2p().en;
    for text in ["Hello. Can you hear me?", "She sells sea shells by the sea shore.", "The quick brown fox jumps over the lazy dog."] {
        assert_eq!(g2p().to_phrases(text), en.to_phrases(text), "{text}");
    }
    let plain = EnglishG2P::new();
    assert_eq!(lyrics_to_syllables("Twinkle, twinkle, little star", g2p()), lyrics_to_syllables("Twinkle, twinkle, little star", en));
    assert!(!plain.is_loaded());
}

#[test]
fn stress_of_common_forms() {
    // forms the mechanically spread stress list had wrong (парке́, ужи́н, ми́нут), and forms with
    // two stresses: the singular (мо́ря, not the plural моря́), the noun (доро́га, not the short
    // adjective дорога́), the full adjective (си́нее, not the comparative сине́е), the locative
    // (в лесу́, not the dative ле́су)
    for (text, ipa) in [
        ("в парке", "fpˈarkʲɪ"),
        ("ужин", "ˈuʐɨn"),
        ("минут", "mʲɪnˈut"),
        ("у моря", "ʊ mˈorʲɐ"),
        ("дорога", "dɐrˈogɐ"),
        ("синее", "sʲˈinʲɪjɪ"),
        ("в лесу", "vlʲɪsˈu"),
        ("в году", "vgɐdˈu"),
        ("уже", "ʊʐˈɛ"),
    ] {
        assert_eq!(said(text), ipa, "{text}");
    }
}

#[test]
fn the_syllable_before_the_stress_is_longer() {
    // Russian rhythm: the syllable right before the stressed one is reduced less than the rest
    // (молоко́ is [məlɐˈko]); it is marked as a secondary stress, which the Klatt rules leave
    // unshortened. Timed like the other unstressed vowels (~50 ms), "река течёт" lost syllables.
    let stresses = |text: &str, speech: bool| -> Vec<u8> { g2p().word(text, speech).iter().filter_map(|s| s.stress).collect() };
    assert_eq!(stresses("молоко", true), [0, 2, 1]);
    assert_eq!(stresses("погода", true), [2, 1, 0]);
    assert_eq!(stresses("мама", true), [1, 0]);
    // sung, the notes time the syllables
    assert_eq!(stresses("молоко", false), [0, 0, 1]);
}
