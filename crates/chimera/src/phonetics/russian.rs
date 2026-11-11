//! Russian text → phoneme segments. The stress comes from the text ("моло+ко", "молоко́", ё), then
//! `public/data/ru-stress.txt.gz` (400k word forms, see `tools/build-ru-stress.py`), then the
//! word's only vowel, then the penultimate syllable. The rest is the standard pronunciation's
//! spelling rules: consonants soft before ь е ё ю я и (ж ш ц never, ч щ й always), я е ё ю as
//! [j]+vowel at a word's start and after a vowel, ь or ъ, и as [ɨ] after ж ш ц, a few spelling
//! exceptions (что, -ого, -тся, сч, стн …), final devoicing and voicing assimilation, also across
//! words, and vowel reduction: in speech unstressed о/а are [ɐ] after hard consonants and е/я/и
//! [ɪ] after soft ones; sung, only о→[a] (akanye), the other vowels full, as singers do.
//!
//! Unstressed о/а are [ɐ], not [ə], also away from the stress: this engine's [ə] is English schwa,
//! calibrated with slightly rounded lips, and Russian listeners heard it as [o] ("на" as "но").

use super::english::Phrase;
use super::syllables::WordSource;
use super::{Intonation, Segment};
use std::collections::HashMap;

pub fn is_cyrillic(c: char) -> bool {
    matches!(c, 'а'..='я' | 'А'..='Я' | 'ё' | 'Ё')
}

pub fn has_cyrillic(s: &str) -> bool {
    s.chars().any(is_cyrillic)
}

pub fn is_vowel_letter(c: char) -> bool {
    matches!(c, 'а' | 'е' | 'ё' | 'и' | 'о' | 'у' | 'ы' | 'э' | 'ю' | 'я')
}

/// a stress mark after the stressed vowel: "+" (моло+ко) or a combining acute (молоко́)
pub fn is_stress_mark(c: char) -> bool {
    c == '+' || c == '\u{301}'
}

/// Joins a vowelless preposition to the next word ("в" + "лесу" → "в|лесу"): one phonological
/// word, but the preposition's consonant stays hard and a following и is [ɨ] ("в избе").
pub const JOIN: char = '|';

/// Unstressed in speech (sung, their vowels are full anyway).
const CLITICS: &[&str] = &[
    "в", "во", "на", "за", "по", "до", "от", "ото", "из", "изо", "у", "к", "ко", "с", "со", "о", "об", "обо", "под", "подо", "над", "при", "про", "без", "для", "и", "а", "но", "не", "ни", "же",
    "ли", "бы", "ль", "б", "ж",
];

/// parts after a hyphen that carry no stress (кто-то, когда-нибудь, давай-ка)
const POSTFIX: &[&str] = &["то", "либо", "нибудь", "ка", "таки", "де", "с"];
/// parts before a hyphen that carry no stress (по-русски, из-за, кое-что)
const PREFIX: &[&str] = &["по", "из", "кое", "кой"];

/// A hyphenated word whose parts are one word: a clitic before or after (кто-то, из-за, по-русски).
/// Otherwise the hyphen joins two words (чижик-пыжик, красно-белый).
pub fn is_clitic_compound(w: &str) -> bool {
    let lower = w.to_lowercase();
    let parts: Vec<&str> = lower.split('-').filter(|p| !p.is_empty()).collect();
    parts.len() > 1 && parts.iter().enumerate().any(|(k, p)| (k > 0 && POSTFIX.contains(p)) || (k + 1 < parts.len() && PREFIX.contains(p)))
}

/// whole words read otherwise than spelled
const SPOKEN: &[(&str, &str)] = &[
    ("что", "што"),
    ("чтобы", "штобы"),
    ("чтоб", "штоб"),
    ("ничто", "ништо"),
    ("нечто", "нешто"),
    ("конечно", "конешно"),
    ("скучно", "скушно"),
    ("нарочно", "нарошно"),
    ("яичница", "яишница"),
    ("сегодня", "севодня"),
    ("сегодняшний", "севодняшний"),
    ("солнце", "сонце"),
    ("здравствуй", "здраствуй"),
    ("здравствуйте", "здраствуйте"),
];

/// -ого/-его is [ovə]/[evə] (the genitive of adjectives and pronouns), except in these
const GO_KEPT: &[&str] = &["много", "немного", "строго", "нестрого", "убого", "полого", "отлого", "дорого", "недорого", "ого"];

/// consonant clusters read otherwise than spelled, inside a word
const CLUSTERS: &[(&str, &str)] = &[
    ("вств", "ств"),
    ("сч", "щ"),
    ("зч", "щ"),
    ("жч", "щ"),
    ("сш", "ш"),
    ("зш", "ш"),
    ("сж", "ж"),
    ("зж", "ж"),
    ("стн", "сн"),
    ("здн", "зн"),
    ("стл", "сл"),
    ("лнц", "нц"),
    ("рдц", "рц"),
    ("ндск", "нск"),
    ("нтск", "нск"),
    ("гк", "хк"),
    ("гч", "хч"),
];

#[derive(Clone, Copy, Debug, PartialEq)]
enum After {
    /// word start, or after a vowel with no [j]
    Start,
    Hard,
    Soft,
    /// after an inserted [j] (я е ё ю at a start, after a vowel, ь or ъ)
    Jot,
}

#[derive(Clone, Debug)]
enum Ph {
    C { ipa: &'static str, soft: bool },
    V { base: char, after: After },
}

fn consonant(c: char) -> Option<&'static str> {
    Some(match c {
        'б' => "b",
        'в' => "v",
        'г' => "g",
        'д' => "d",
        'ж' => "ʐ",
        'з' => "z",
        'й' => "j",
        'к' => "k",
        'л' => "l",
        'м' => "m",
        'н' => "n",
        'п' => "p",
        'р' => "r",
        'с' => "s",
        'т' => "t",
        'ф' => "f",
        'х' => "x",
        'ц' => "ts",
        'ч' => "tɕ",
        'ш' => "ʂ",
        'щ' => "ɕ",
        _ => return None,
    })
}

/// ж ш ц: never soft
fn hard_only(ipa: &str) -> bool {
    matches!(ipa, "ʐ" | "ʂ" | "ts")
}

/// ч щ й: soft by themselves
fn soft_only(ipa: &str) -> bool {
    matches!(ipa, "tɕ" | "ɕ" | "j")
}

fn consonant_ipa(base: &str, soft: bool) -> String {
    if base == "l" {
        return if soft { "lʲ" } else { "ɫ" }.to_string();
    }
    if soft && matches!(base, "p" | "b" | "t" | "d" | "m" | "n" | "f" | "v" | "s" | "z" | "r" | "k" | "g" | "x") {
        return format!("{base}ʲ");
    }
    base.to_string()
}

const VOICING: [(&str, &str); 10] = [("b", "p"), ("d", "t"), ("g", "k"), ("v", "f"), ("z", "s"), ("ʐ", "ʂ"), ("ʑ", "ɕ"), ("dʑ", "tɕ"), ("dz", "ts"), ("ɣ", "x")];

fn split_soft(ipa: &str) -> (&str, &str) {
    match ipa.strip_suffix('ʲ') {
        Some(b) => (b, "ʲ"),
        None => (ipa, ""),
    }
}

fn is_voiced_obstruent(ipa: &str) -> bool {
    let (b, _) = split_soft(ipa);
    VOICING.iter().any(|(v, _)| *v == b)
}

fn is_voiceless_obstruent(ipa: &str) -> bool {
    let (b, _) = split_soft(ipa);
    VOICING.iter().any(|(_, u)| *u == b)
}

fn devoice(ipa: &str) -> String {
    let (b, s) = split_soft(ipa);
    VOICING.iter().find(|(v, _)| *v == b).map_or(ipa.to_string(), |(_, u)| format!("{u}{s}"))
}

fn voice(ipa: &str) -> String {
    let (b, s) = split_soft(ipa);
    VOICING.iter().find(|(_, u)| *u == b).map_or(ipa.to_string(), |(v, _)| format!("{v}{s}"))
}

/// A voiced obstruent before a voiceless one is devoiced, a voiceless one before a voiced one
/// (other than [v]) voiced: "лодка" [tk], "сделать" [zd], "вчера" [ftɕ]. `segs[i]` before `next`.
fn assimilate(seg: &mut Segment, next: &str) {
    if is_voiceless_obstruent(next) && is_voiced_obstruent(&seg.ipa) {
        seg.ipa = devoice(&seg.ipa);
    } else if is_voiced_obstruent(next) && split_soft(next).0 != "v" && is_voiceless_obstruent(&seg.ipa) {
        seg.ipa = voice(&seg.ipa);
    }
}

fn reduced(base: char, after: After, last: bool, speech: bool) -> &'static str {
    if !speech {
        // sung: akanye stays (о → a), the other vowels keep their quality
        return match base {
            'a' | 'o' => "a",
            'u' => "u",
            'ɨ' => "ɨ",
            'e' | 'ɛ' => "e",
            _ => "i",
        };
    }
    match (base, after) {
        ('a' | 'o', After::Hard | After::Start) => "ɐ",
        // я, ё, а after ч щ: [ɪ], but a final я is [ə]-like (дядя): [ɐ] here (see the module doc)
        ('a' | 'o', _) => {
            if last {
                "ɐ"
            } else {
                "ɪ"
            }
        }
        // е after ж ш ц: жена [ʐɨˈna]
        ('e' | 'ɛ', After::Hard) => "ɨ",
        ('e' | 'ɛ', _) => "ɪ",
        ('i', After::Hard) => "ɨ",
        ('i', _) => "ɪ",
        ('ɨ', _) => "ɨ",
        _ => "ʊ",
    }
}

fn stressed(base: char) -> &'static str {
    match base {
        'a' => "a",
        'o' => "o",
        'u' => "u",
        'ɨ' => "ɨ",
        'ɛ' => "ɛ",
        'e' => "e",
        _ => "i",
    }
}

/// Spelling exceptions on one word (no [`JOIN`]): whole words, -ого/-его, -тся/-ться, clusters.
fn respell(w: &str) -> String {
    if let Some((_, s)) = SPOKEN.iter().find(|(a, _)| *a == w) {
        return s.to_string();
    }
    let mut s = w.to_string();
    if (s.ends_with("ого") || s.ends_with("его")) && !GO_KEPT.contains(&w) && s.chars().count() >= 3 {
        let n = s.len() - "го".len();
        s = format!("{}во", &s[..n]);
    }
    if let Some(p) = s.strip_suffix("ться").or_else(|| s.strip_suffix("тся")) {
        s = format!("{p}ца");
    }
    for (a, b) in CLUSTERS {
        s = s.replace(a, b);
    }
    s
}

#[derive(Clone, Debug, Default)]
pub struct RussianG2P {
    /// е-spelled form → (spelling with ё, stressed vowel 1-based)
    dict: Option<HashMap<String, (String, u8)>>,
}

impl RussianG2P {
    pub fn new() -> RussianG2P {
        RussianG2P::default()
    }

    pub fn is_loaded(&self) -> bool {
        self.dict.is_some()
    }

    /// The stress dictionary as text ("spelling\tn" lines: `public/data/ru-stress.txt.gz` gunzipped).
    pub fn load_dict(&mut self, text: &str) {
        let mut m = HashMap::with_capacity(410_000);
        for line in text.lines() {
            let Some((w, n)) = line.split_once('\t') else { continue };
            let Ok(n) = n.trim().parse::<u8>() else { continue };
            m.insert(w.replace('ё', "е"), (w.to_string(), n));
        }
        self.dict = Some(m);
    }

    /// One word's spelling (ё restored from the dictionary) and stressed vowel (0-based), from the
    /// marks in it, ё, the dictionary, its only vowel, or the penultimate.
    fn stress(&self, w: &str) -> (String, Option<usize>) {
        let mut letters = String::new();
        let mut marked = None;
        let mut nv = 0;
        for c in w.chars() {
            if is_stress_mark(c) {
                if letters.chars().last().is_some_and(is_vowel_letter) {
                    marked = Some(nv - 1);
                }
                continue;
            }
            if is_vowel_letter(c) {
                nv += 1;
            }
            letters.push(c);
        }
        if marked.is_some() {
            return (letters, marked);
        }
        if let Some(k) = letters.chars().filter(|c| is_vowel_letter(*c)).position(|c| c == 'ё') {
            return (letters, Some(k));
        }
        // the dictionary knows the word after the joined prepositions
        let (pre, word) = match letters.rfind(JOIN) {
            Some(i) => (letters[..=i].to_string(), letters[i + JOIN.len_utf8()..].to_string()),
            None => (String::new(), letters.clone()),
        };
        if let Some((spelled, n)) = self.dict.as_ref().and_then(|d| d.get(&word)) {
            return (format!("{pre}{spelled}"), Some(*n as usize - 1));
        }
        match nv {
            0 => (letters, None),
            1 => (letters, Some(0)),
            n => (letters, Some(n - 2)),
        }
    }

    /// One word (it may hold [`JOIN`]s and hyphens) → segments. `reduce`: speech (vowel
    /// reduction, unstressed clitics); sung otherwise.
    pub fn word(&self, w: &str, reduce: bool) -> Vec<Segment> {
        let lower = w.to_lowercase();
        let clean = lower.trim_matches(|c: char| c == '-' || c == '\'');
        if clean.contains('-') && !self.dict.as_ref().is_some_and(|d| d.contains_key(&clean.replace('ё', "е"))) {
            let parts: Vec<&str> = clean.split('-').filter(|p| !p.is_empty()).collect();
            let mut out = Vec::new();
            for (k, p) in parts.iter().enumerate() {
                let bare: String = p.chars().filter(|c| !is_stress_mark(*c)).collect();
                let weak = (k > 0 && POSTFIX.contains(&bare.as_str())) || (k + 1 < parts.len() && PREFIX.contains(&bare.as_str()));
                out.extend(self.word_part(p, reduce, weak));
            }
            return devoice_final(out);
        }
        let bare: String = clean.chars().filter(|c| !is_stress_mark(*c)).collect();
        let weak = reduce && CLITICS.contains(&bare.as_str());
        devoice_final(self.word_part(&clean.replace('-', ""), reduce, weak))
    }

    /// `weak`: no stress (a clitic, or the unstressed part of a hyphenated word)
    fn word_part(&self, w: &str, reduce: bool, weak: bool) -> Vec<Segment> {
        let (spelled, stress) = self.stress(w);
        let stress = if weak { None } else { stress };
        // spelling exceptions, part by part (they change no vowels, so the stress stays put)
        let spelled: String = spelled.split(JOIN).map(respell).collect::<Vec<_>>().join(&JOIN.to_string());
        let s: Vec<char> = spelled.chars().filter(|c| is_cyrillic(*c) || *c == JOIN).flat_map(char::to_lowercase).collect();
        let mut ph: Vec<Ph> = Vec::new();
        let (mut jot, mut boundary) = (false, false);
        for (i, &c) in s.iter().enumerate() {
            if c == JOIN {
                boundary = true;
                continue;
            }
            if c == 'ь' || c == 'ъ' {
                jot = true;
                continue;
            }
            if is_vowel_letter(c) {
                let (mut base, iotated) = match c {
                    'а' => ('a', false),
                    'о' => ('o', false),
                    'у' => ('u', false),
                    'ы' => ('ɨ', false),
                    'э' => ('ɛ', false),
                    'е' => ('e', true),
                    'ё' => ('o', true),
                    'ю' => ('u', true),
                    'я' => ('a', true),
                    _ => ('i', false),
                };
                let prev_c = match ph.last() {
                    Some(Ph::C { ipa, .. }) if !jot => Some(*ipa),
                    _ => None,
                };
                let after = match prev_c {
                    Some(p) if boundary => {
                        // a joined preposition before a vowel: "в избе" [vɨ], "с ёлки" [sjo]
                        if c == 'и' {
                            base = 'ɨ';
                            After::Hard
                        } else if iotated {
                            ph.push(Ph::C { ipa: "j", soft: false });
                            After::Jot
                        } else {
                            let _ = p;
                            After::Hard
                        }
                    }
                    Some(p) if hard_only(p) => {
                        base = match c {
                            'и' => 'ɨ',
                            'е' => 'ɛ',
                            _ => base,
                        };
                        After::Hard
                    }
                    Some(p) if iotated || c == 'и' || soft_only(p) => After::Soft,
                    Some(_) => After::Hard,
                    None => {
                        if iotated || (c == 'и' && jot) {
                            ph.push(Ph::C { ipa: "j", soft: false });
                            After::Jot
                        } else {
                            After::Start
                        }
                    }
                };
                ph.push(Ph::V { base, after });
                jot = false;
                boundary = false;
                continue;
            }
            let Some(base) = consonant(c) else { continue };
            let next = s.get(i + 1).copied();
            let soft = !hard_only(base) && !soft_only(base) && matches!(next, Some('ь' | 'е' | 'ё' | 'ю' | 'я' | 'и'));
            // a doubled consonant is one (касса, ванна)
            if let Some(Ph::C { ipa, soft: s0 }) = ph.last()
                && *ipa == base
                && !boundary
            {
                let s0 = *s0;
                ph.pop();
                ph.push(Ph::C { ipa: base, soft: soft || s0 });
                continue;
            }
            ph.push(Ph::C { ipa: base, soft });
            jot = false;
            boundary = false;
        }
        let n_vowels = ph.iter().filter(|p| matches!(p, Ph::V { .. })).count();
        let mut out: Vec<Segment> = Vec::with_capacity(ph.len());
        let mut k = 0;
        for p in &ph {
            match p {
                Ph::C { ipa, soft } => out.push(Segment::new(&consonant_ipa(ipa, *soft))),
                Ph::V { base, after } => {
                    let is_stressed = stress == Some(k);
                    let ipa = if is_stressed { stressed(*base) } else { reduced(*base, *after, k + 1 == n_vowels, reduce) };
                    out.push(Segment { ipa: ipa.to_string(), stress: Some(if is_stressed { 1 } else { 0 }), ..Default::default() });
                    k += 1;
                }
            }
        }
        // voicing assimilation, right to left
        for i in (0..out.len().saturating_sub(1)).rev() {
            let next = out[i + 1].ipa.clone();
            assimilate(&mut out[i], &next);
        }
        out
    }

    /// Text → phrases of segments: split at punctuation as English text is; Latin words go to
    /// `latin` (with no English allophones: they would flap Russian /t/ between vowels).
    pub fn to_phrases(&self, text: &str, latin: &impl WordSource) -> Vec<Phrase> {
        let mut phrases = Vec::new();
        let mut words: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut flush = |delim: &str, cur: &mut String, phrases: &mut Vec<Phrase>| {
            for w in cur.split(|c: char| !(c.is_alphabetic() || c == '-' || c == '\'' || is_stress_mark(c))) {
                if !w.trim_matches('-').is_empty() {
                    words.push(w.to_lowercase());
                }
            }
            cur.clear();
            if words.is_empty() {
                return;
            }
            // vowelless prepositions join the next word
            let mut merged: Vec<String> = Vec::new();
            let mut pre: Option<String> = None;
            for w in words.drain(..) {
                let vowelless = has_cyrillic(&w) && !w.chars().any(is_vowel_letter) && w.chars().count() <= 2;
                match pre.take() {
                    Some(p) if has_cyrillic(&w) && !vowelless => merged.push(format!("{p}{JOIN}{w}")),
                    Some(p) => {
                        merged.push(p);
                        if vowelless { pre = Some(w) } else { merged.push(w) }
                    }
                    None if vowelless => pre = Some(w),
                    None => merged.push(w),
                }
            }
            merged.extend(pre);
            let mut segs: Vec<Segment> = Vec::new();
            let mut said: Vec<String> = Vec::new();
            for w in &merged {
                let mut ws = if has_cyrillic(w) { self.word(w, true) } else { latin.word(w, true) };
                let Some(first) = ws.first().map(|s| s.ipa.clone()) else { continue };
                // across the word boundary: "сад был" [d b], "к дому" [g d]
                if let Some(prev) = segs.last_mut() {
                    assimilate(prev, &first);
                }
                ws.last_mut().unwrap().word_end = Some(true);
                segs.append(&mut ws);
                said.push(w.replace(JOIN, " "));
            }
            let q = delim.contains('?');
            let pause = if delim.contains(['.', '!', '?', '…']) {
                0.35
            } else if !delim.is_empty() {
                0.18
            } else {
                0.3
            };
            if !segs.is_empty() {
                phrases.push(Phrase { segments: segs, intonation: if q { Intonation::Question } else { Intonation::Statement }, pause, words: said });
            }
        };
        let mut delim = String::new();
        for c in text.chars() {
            if is_phrase_punct(c) {
                delim.push(c);
                continue;
            }
            if !delim.is_empty() {
                flush(&delim, &mut cur, &mut phrases);
                delim.clear();
            }
            cur.push(c);
        }
        flush(&delim, &mut cur, &mut phrases);
        phrases
    }
}

fn is_phrase_punct(c: char) -> bool {
    matches!(c, '.' | '!' | '?' | ';' | ':' | ',' | '…' | '—' | '–')
}

/// a word-final voiced obstruent is voiceless (дуб [p], снег [k])
fn devoice_final(mut segs: Vec<Segment>) -> Vec<Segment> {
    if let Some(last) = segs.last_mut()
        && is_voiced_obstruent(&last.ipa)
    {
        last.ipa = devoice(&last.ipa);
        // and the obstruents before it follow (визг [sk])
        for i in (0..segs.len().saturating_sub(1)).rev() {
            let next = segs[i + 1].ipa.clone();
            assimilate(&mut segs[i], &next);
        }
    }
    segs
}

impl WordSource for RussianG2P {
    fn word(&self, w: &str, reduce: bool) -> Vec<Segment> {
        RussianG2P::word(self, w, reduce)
    }
}

/// A Russian word's spelling split into `n` sung syllables, as the sounds are: the consonants
/// between two vowels open the next (пе-сня), but a sonorant (й л м н р) before another consonant
/// closes the one before (вой-на, фон-тан-ке, боль-ше); ь ъ stay with the consonant before them.
/// None when the vowels don't number `n`.
pub fn split_spelling(word: &str, n: usize) -> Option<Vec<String>> {
    let s: Vec<char> = word.chars().collect();
    let vi: Vec<usize> = s.iter().enumerate().filter(|(_, c)| is_vowel_letter(c.to_lowercase().next().unwrap_or(**c))).map(|(i, _)| i).collect();
    if vi.len() != n || n == 0 {
        return None;
    }
    let mut out = Vec::with_capacity(n);
    let mut start = 0;
    for (k, &v) in vi.iter().enumerate() {
        let end = match vi.get(k + 1) {
            Some(&next) => {
                let low = |i: usize| s[i].to_lowercase().next().unwrap_or(s[i]);
                let mut e = v + 1;
                // a sonorant (and its ь) closes the syllable when a consonant still follows
                while e < next && matches!(low(e), 'й' | 'л' | 'м' | 'н' | 'р') {
                    let mut f = e + 1;
                    while f < next && matches!(low(f), 'ь' | 'ъ') {
                        f += 1;
                    }
                    if f >= next {
                        break;
                    }
                    e = f;
                }
                e
            }
            None => s.len(),
        };
        out.push(s[start..end].iter().collect());
        start = end;
    }
    Some(out)
}
