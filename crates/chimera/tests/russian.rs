//! Russian through IPA: palatalized ("soft") consonants, the alveolo-palatals /ɕ ʑ tɕ dʑ/, and
//! bracketed IPA lyrics for singing.
use chimera::anatomy::human::{ArticKey, BodyMix};
use chimera::phonetics::Segment;
use chimera::phonetics::inventory::{Manner, phoneme};
use chimera::phonetics::ipa::tokenize_ipa;
use chimera::phonetics::planner::{PlanOptions, plan_utterance};
use chimera::phonetics::syllables::lyrics_to_syllables;

fn ipas(segs: &[Segment]) -> Vec<String> {
    segs.iter().map(|s| s.ipa.clone()).collect()
}

fn joined(segs: &[Segment]) -> String {
    segs.iter().map(|s| s.ipa.as_str()).collect()
}

#[test]
fn soft_mark_glues_to_its_consonant() {
    // "люблю"
    assert_eq!(ipas(&tokenize_ipa("lʲʊˈblʲu")), ["lʲ", "ʊ", "b", "lʲ", "u"]);
    // "мать" vs "мат": the final consonant differs
    assert_eq!(ipas(&tokenize_ipa("matʲ")), ["m", "a", "tʲ"]);
    assert_eq!(ipas(&tokenize_ipa("mat")), ["m", "a", "t"]);
    // stress still lands on the vowel after a soft consonant
    let s = tokenize_ipa("prʲɪˈvʲet");
    assert_eq!(ipas(&s), ["p", "rʲ", "ɪ", "vʲ", "e", "t"]);
    assert_eq!(s[4].stress, Some(1));
}

#[test]
fn alveolo_palatals_tokenize() {
    // "щи", "чай" (with and without the tie bar), "дочь" as dictionaries write it ("tɕʲ")
    assert_eq!(ipas(&tokenize_ipa("ɕːi")), ["ɕ", "i"]);
    assert!(tokenize_ipa("ɕːi")[0].dur.is_some(), "ː lengthens ɕ");
    assert_eq!(ipas(&tokenize_ipa("tɕaj")), ["tɕ", "a", "j"]);
    assert_eq!(ipas(&tokenize_ipa("t͡ɕaj")), ["tɕ", "a", "j"]);
    assert_eq!(ipas(&tokenize_ipa("dot͡ɕʲ")), ["d", "o", "tɕ"]);
    // a soft mark on a consonant with no soft form is dropped, the consonant kept
    assert_eq!(ipas(&tokenize_ipa("ʂʲ")), ["ʂ"]);
    // щ written as a soft /ʃ/ ("ʃʲːuka") is /ɕ/, not a hard /ʃ/
    assert_eq!(ipas(&tokenize_ipa("ʃʲːukə")), ["ɕ", "u", "k", "ə"]);
    assert_eq!(ipas(&tokenize_ipa("ʒʲːi")), ["ʑ", "i"]);
    // and ч written as a soft /tʃ/ is /tɕ/
    assert_eq!(ipas(&tokenize_ipa("tʃʲaj")), ["tɕ", "a", "j"]);
}

#[test]
fn derived_consonants_are_not_calibrated_themselves() {
    // `chimera calibrate consonants` fits every consonant with an `area`; the soft forms and the
    // alveolo-palatals borrow their source's fit instead, so the committed table stays as it is
    // (the CLI test `calibrate_consonants_reproduces_the_committed_table` failed without this)
    for ipa in ["pʲ", "bʲ", "tʲ", "dʲ", "mʲ", "nʲ", "fʲ", "vʲ", "sʲ", "zʲ", "lʲ", "rʲ", "ɾʲ", "kʲ", "gʲ", "xʲ", "ɕ", "ʑ", "tɕ", "dʑ"] {
        let d = phoneme(ipa, BodyMix::MALE).unwrap();
        assert_eq!((d.area, d.cal_key), (None, None), "{ipa}");
    }
}

#[test]
fn soft_consonants_raise_the_tongue_body_and_keep_their_constriction() {
    let body = BodyMix::MALE;
    for soft in ["pʲ", "bʲ", "tʲ", "dʲ", "mʲ", "nʲ", "fʲ", "vʲ", "sʲ", "zʲ", "lʲ", "rʲ", "ɾʲ"] {
        let plain = soft.strip_suffix('ʲ').unwrap();
        let (p, s) = (phoneme(plain, body).unwrap(), phoneme(soft, body).unwrap());
        assert_eq!(p.manner, s.manner, "{soft}");
        assert_eq!(p.voicing, s.voicing, "{soft}");
        assert_eq!(p.dominant, s.dominant, "{soft}");
        assert!(s.targets.get(ArticKey::TbY).unwrap() > 7.5, "{soft} raises the tongue body");
        // the calibrated primary constriction is the plain consonant's
        for k in p.dominant {
            assert_eq!(p.targets.get(*k), s.targets.get(*k), "{soft} {k:?}");
        }
    }
    // soft velars close at the palatal place
    for (soft, palatal) in [("kʲ", "c"), ("gʲ", "ɟ"), ("xʲ", "ç")] {
        let (s, p) = (phoneme(soft, body).unwrap(), phoneme(palatal, body).unwrap());
        assert_eq!(s.targets, p.targets, "{soft}");
        assert_eq!(s.manner, p.manner, "{soft}");
    }
    let sh = phoneme("ɕ", body).unwrap();
    assert_eq!(sh.manner, Manner::Fricative);
    // /ɕ/ borrows /ʃ/'s calibrated tip height, but keeps its lips flatter than /ʃ/
    assert_eq!(sh.targets.get(ArticKey::TtY), phoneme("ʃ", body).unwrap().targets.get(ArticKey::TtY));
    assert!(sh.targets.get(ArticKey::LipProt).unwrap() < phoneme("ʃ", body).unwrap().targets.get(ArticKey::LipProt).unwrap());
    assert_eq!(phoneme("tɕ", body).unwrap().release, Some("ɕ"));
    assert_eq!(phoneme("dʑ", body).unwrap().release, Some("ʑ"));
}

#[test]
fn russian_phrase_plans() {
    // "я тебя люблю", "щука", "чей", "мать"
    for text in ["ja tʲɪˈbʲa lʲʊˈblʲu", "ˈɕːukə", "tɕej", "matʲ", "ˈkʲit xʲɪˈmʲik"] {
        let segs = tokenize_ipa(text);
        let plan = plan_utterance(&segs, &PlanOptions { f0: Some(110.0), ..Default::default() });
        assert!(plan.end > 0.0 && !plan.gestures.is_empty(), "{text}");
        // every segment is planned as a known kind (no "?" for an unknown phoneme)
        assert!(plan.segments.iter().all(|s| s.kind != "?"), "{text}: {:?}", plan.segments);
        assert_eq!(plan.segments.len(), segs.len(), "{text}");
    }
}

fn no_dict(_: &str, _: bool) -> Vec<Segment> {
    Vec::new()
}

#[test]
fn bracketed_ipa_lyrics_with_breaks() {
    let syl = lyrics_to_syllables("песня[ˈpʲe.sʲnʲə]", &no_dict);
    assert_eq!(syl.len(), 2);
    assert_eq!(syl.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(), ["пес", "ня"]);
    assert_eq!((joined(&syl[0].onset), syl[0].nucleus.ipa.as_str(), joined(&syl[0].coda)), ("pʲ".into(), "e", "".into()));
    assert_eq!((joined(&syl[1].onset), syl[1].nucleus.ipa.as_str()), ("sʲnʲ".into(), "ə"));
    assert_eq!(syl[0].nucleus.stress, Some(1));
    assert!(!syl[0].word_end && syl[1].word_end);
    // display hyphens pick the split
    let syl = lyrics_to_syllables("пе-сня[ˈpʲe.sʲnʲə]", &no_dict);
    assert_eq!(syl.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(), ["пе", "сня"]);
}

#[test]
fn bracketed_ipa_lyrics_open_syllables_by_default() {
    // "звезда": the cluster goes to the next syllable
    let syl = lyrics_to_syllables("[zvʲɪzˈda]", &no_dict);
    assert_eq!(syl.iter().map(|s| joined(&s.onset)).collect::<Vec<_>>(), ["zvʲ", "zd"]);
    // with no display text the notes show the IPA
    assert_eq!(syl.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(), ["zvʲɪ", "zda"]);
    // /j/ before a consonant closes its syllable ("вой-на"), alone it opens the next ("ма-йор")
    let syl = lyrics_to_syllables("[vɐjˈna] [mɐˈjor]", &no_dict);
    assert_eq!(syl.len(), 4);
    assert_eq!(joined(&syl[0].coda), "j");
    assert_eq!(joined(&syl[1].onset), "n");
    assert_eq!(joined(&syl[2].coda), "");
    assert_eq!(joined(&syl[3].onset), "j");
    assert!(syl[1].word_end && syl[3].word_end);
}

#[test]
fn bracketed_ipa_mixes_with_punctuation_and_melisma() {
    let syl = lyrics_to_syllables("мама[ˈma.mə] - мыла[ˈmɨ.ɫə], раму[ˈra.mʊ].", &no_dict);
    assert_eq!(syl.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(), ["ма", "ма", "~", "мы", "ла", "ра", "му"]);
    assert_eq!(syl[2].melisma, Some(true));
    assert!(syl[4].phrase_end && syl[6].phrase_end);
    assert!(!syl[1].phrase_end);
}
