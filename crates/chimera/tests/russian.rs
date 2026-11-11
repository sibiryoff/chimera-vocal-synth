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
    // `chimera calibrate consonants` fits every consonant with an `area`; the soft forms borrow
    // their plain form's fit instead (the CLI test `calibrate_consonants_reproduces_the_committed_table`
    // failed when they carried one). /ɕ ʑ/ have their own geometry and their own fit.
    for ipa in ["pʲ", "bʲ", "tʲ", "dʲ", "mʲ", "nʲ", "fʲ", "vʲ", "sʲ", "zʲ", "lʲ", "rʲ", "ɾʲ", "kʲ", "gʲ", "xʲ", "tɕ", "dʑ"] {
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
    // /ɕ/ has its own calibrated tip height (a narrower slit than /ʃ/'s) and lips flatter than /ʃ/
    assert!((sh.area, sh.cal_key) == (Some(0.15), Some(ArticKey::TtY)));
    assert_ne!(sh.targets.get(ArticKey::TtY), phoneme("ʃ", body).unwrap().targets.get(ArticKey::TtY));
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

// ---------------------------------------------------------------- the sibilants' noise

/// "ˈaCa" rendered; for consonant `c`: its 1–11 kHz noise level (dB re the stressed vowel's total
/// power) and the noise's centre of gravity (Hz), both over the middle 60% of the consonant.
fn sibilant(c: &str, body: &str, f0: f64) -> (f64, f64) {
    use chimera::voice::human_voice::{HumanVoice, SourcePatch};
    const FS: f64 = 48000.0;
    const N: usize = 1024;
    let plan = plan_utterance(&tokenize_ipa(&format!("ˈa{c}a")), &PlanOptions { body: body.into(), t0: 0.05, f0: Some(f0), ..Default::default() });
    let mut v = HumanVoice::new(FS, if body == "male" { chimera::anatomy::human::BODY_MALE } else { chimera::anatomy::human::BODY_FEMALE }, 2);
    v.set_source(&SourcePatch::f0(f0));
    v.schedule(&plan.gestures);
    let mut x = vec![0.0f32; (FS * (plan.end + 0.3)).round() as usize];
    for chunk in x.chunks_mut(128) {
        v.render(chunk);
    }
    // summed power spectrum (Hann, hop N/2) over the middle 60% of a segment; bin k is k·FS/N Hz
    let spectrum = |t0: f64, t1: f64| -> Vec<f64> {
        let (a, b) = (((t0 + 0.2 * (t1 - t0)) * FS) as usize, ((t1 - 0.2 * (t1 - t0)) * FS) as usize);
        let w: Vec<f64> = (0..N).map(|i| 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / N as f64).cos()).collect();
        let mut p = vec![0.0; N / 2];
        let mut i = a;
        while i + N <= b.max(a + N) && i + N <= x.len() {
            for (k, pk) in p.iter_mut().enumerate() {
                let (mut re, mut im) = (0.0, 0.0);
                for (j, wj) in w.iter().enumerate() {
                    let ph = std::f64::consts::TAU * (k * j) as f64 / N as f64;
                    let s = x[i + j] as f64 * wj;
                    re += s * ph.cos();
                    im -= s * ph.sin();
                }
                *pk += re * re + im * im;
            }
            i += N / 2;
        }
        p
    };
    let seg = |ipa: &str| plan.segments.iter().find(|s| s.ipa == ipa).map(|s| (s.t0, s.t1)).unwrap_or_else(|| panic!("no {ipa} in the plan"));
    let (v0, v1) = seg("a");
    let (c0, c1) = seg(c);
    let vowel: f64 = spectrum(v0, v1).iter().sum();
    let p = spectrum(c0, c1);
    let band = |k: usize| (k as f64 * FS / N as f64, p[k]);
    let (mut e, mut m) = (0.0, 0.0);
    for (f, pk) in (0..N / 2).map(band).filter(|(f, _)| (1000.0..11000.0).contains(f)) {
        e += pk;
        m += f * pk;
    }
    (10.0 * (e / vowel).log10(), m / e)
}

#[test]
#[ignore = "slow in debug builds: cargo test --release -p chimera --test russian -- --ignored"]
fn russian_sibilants_are_ordered_and_audible() {
    // Russian /ʂ/ (ш) is the lowest of the sibilants, /ɕ/ (щ) between it and /s/ (Kochetov 2017,
    // "Acoustics of Russian voiceless sibilant fricatives", JIPA 47(3)); /ʂ ʐ/ were 12 and 10 dB
    // quieter than /s z/ (the incisor source faded: the curled tip sat too far behind the teeth)
    for (body, f0) in [("male", 112.0), ("female", 205.0)] {
        let m: Vec<(&str, (f64, f64))> = ["s", "ɕ", "ʃ", "ʂ", "z", "ʐ"].iter().map(|c| (*c, sibilant(c, body, f0))).collect();
        for (c, (db, cog)) in &m {
            println!("{body} {c}: {db:6.1} dB  {:.2} kHz", cog / 1000.0);
        }
        let get = |c: &str| m.iter().find(|(x, _)| *x == c).unwrap().1;
        let (s, sh_ru, sch, sh_en, z, zh) = (get("s"), get("ʂ"), get("ɕ"), get("ʃ"), get("z"), get("ʐ"));
        assert!(sh_ru.1 + 500.0 < sch.1 && sch.1 + 300.0 < s.1, "{body}: ш < щ < с by centre of gravity");
        assert!(sh_ru.1 < sh_en.1, "{body}: Russian ш is lower than English sh");
        assert!(sh_ru.0 > s.0 - 6.0 && sch.0 > s.0 - 6.0, "{body}: ш and щ within 6 dB of с");
        assert!(zh.0 > z.0 - 6.0, "{body}: ж within 6 dB of з");
    }
}

// ---------------------------------------------------------------- ы, and phrases sung back to back

#[test]
fn close_central_vowel_is_unrounded() {
    // ы (/ɨ/): its steady part is a slightly retracted [i] with /i/'s lips. Calibrated with a lip
    // slit (lipAp 0.2), then with protruded narrowed lips, at a textbook close-central F2 of 1600,
    // it was heard as rounded ("ты" as "тю", then between у and ю).
    use chimera::phonetics::planner::vowel_artic;
    for body in [BodyMix::MALE, BodyMix::FEMALE, BodyMix::CHILD] {
        let (y, i, w) = (vowel_artic("ɨ", body).unwrap(), vowel_artic("i", body).unwrap(), vowel_artic("ɯ", body).unwrap());
        let g = |p: &chimera::anatomy::human::ArticPatch, k: ArticKey| p.get(k).unwrap();
        assert!(g(&y, ArticKey::LipAp) >= 0.98 && g(&y, ArticKey::LipWidth) >= 0.94 && g(&y, ArticKey::LipProt) <= 0.03, "{body:?}: /i/'s lips");
        assert!(g(&w, ArticKey::TbX) < g(&y, ArticKey::TbX) && g(&y, ArticKey::TbX) < g(&i, ArticKey::TbX), "{body:?}: the tongue between [ɯ] and [i]");
    }
}

#[test]
fn hard_consonant_before_y_is_velarized() {
    // ы is [ˠi] (Padgett, Bennett et al. 2024): the back tongue is the consonant's. An onglide
    // inside the vowel ([ɯ] then [i]) was heard as two vowels, "ыи".
    let velarized = |ipa: &str| {
        let p = plan_utterance(&tokenize_ipa(ipa), &PlanOptions { body: BodyMix::MALE, t0: 0.0, ..Default::default() });
        assert_eq!(p.segments.iter().filter(|s| s.kind == "vowel").count(), 1, "{ipa}: one vowel");
        p.gestures.iter().filter(|g| g.label.as_ref().is_some_and(|l| l.as_str() == "velarization")).count()
    };
    assert_eq!(velarized("ˈtɨ"), 1);
    assert_eq!(velarized("ˈbɨɫ"), 1);
    assert_eq!(velarized("ˈʐɨk"), 1);
    assert_eq!(velarized("ˈtʲi"), 0);
    assert_eq!(velarized("ˈti"), 0);
    assert_eq!(velarized("ˈtu"), 0);
}

#[test]
fn phrases_sung_back_to_back_keep_their_onsets_voiced() {
    // "…был? На Фонтанке…": each sung phrase is planned as its own utterance, and its
    // end-of-breath-group exhale (an abduction for 0.1 s after the phrase, plus its ramp) outlasted
    // the 0.12 s left before the next phrase and spread over the next phrase's onset: the /n/ of
    // "на" came out 28 dB below the other /n/'s, unvoiced. No exhale may overlap a sung segment;
    // the last phrase (followed by a rest) still ends in one.
    use chimera::phonetics::english::EnglishG2P;
    use chimera::phonetics::sing::{SongOptions, SungNote, plan_song};
    let g = EnglishG2P::default();
    for (lyrics, n) in [
        ("Чижик[ˈtɕi.ʐɨk] пыжик[ˈpɨ.ʐɨk], где[gdʲe] ты[tɨ] был[bɨɫ]? На[na] Фонтанке[fɐn.ˈtan.kʲɪ] водку[ˈvot.ku] пил[pʲiɫ].", 14),
        ("Breathe in, breathe out, I am made of air.", 9),
    ] {
        let syls = lyrics_to_syllables(lyrics, &g);
        assert_eq!(syls.len(), n, "{lyrics}");
        let notes: Vec<SungNote> = syls.into_iter().enumerate().map(|(k, syl)| SungNote { midi: 55.0 + (k % 5) as f64, beats: 1.0, syl }).collect();
        let plan = plan_song(&notes, &SongOptions { body: BodyMix::MALE, bpm: 120.0, t0: 0.3, vibrato: 0.35 });
        let exhales: Vec<_> = plan.gestures.iter().filter(|g| g.label.as_ref().is_some_and(|l| l.as_str() == "exhale")).collect();
        assert_eq!(exhales.len(), 1, "{lyrics}: one exhale, at the end");
        for e in &exhales {
            for s in &plan.segments {
                assert!(!(s.t0 < e.t1 + e.ramp_out && s.t1 > e.t0), "{lyrics}: the exhale at {:.3} overlaps /{}/ at {:.3}", e.t0, s.ipa, s.t0);
            }
        }
    }
}
