//! Lyrics → sung syllables. Words come from the pronouncing dictionary (unreduced: sung function
//! words keep full vowels), are split into syllables by the maximal-onset principle with English
//! onset clusters, and the spelling is split to match for display ("twin-kle").
//! Typed hyphens force syllable breaks; a lone "-" (or "~") holds the previous vowel onto the
//! next note (melisma), as in vocaloid editors.
//!
//! IPA in square brackets bypasses the dictionary, for any language: `[ˈpʲe.sʲnʲə]`, or with the
//! text to show on the notes in front, `песня[ˈpʲe.sʲnʲə]`. Dots (or hyphens) mark syllable
//! breaks; without them the consonants between two vowels start the next syllable (open
//! syllables, as sung), except a /j/ before another consonant ("voj.na").

use crate::anatomy::human::BodyMix;
use super::english::js_is_space;
use super::ipa::tokenize_ipa;
use super::planner::is_vowel;
use super::russian::{self, JOIN, has_cyrillic, is_cyrillic, is_stress_mark};
use super::{Segment, Syllable};
use serde::{Deserialize, Serialize};

// English onsets (IPA), longest first matters only for the search below
#[rustfmt::skip]
const ONSETS: &[&str] = &[
    "p", "b", "t", "d", "k", "g", "f", "v", "θ", "ð", "s", "z", "ʃ", "ʒ", "h", "m", "n", "l", "ɹ", "w", "j", "tʃ", "dʒ",
    "pl", "bl", "kl", "gl", "fl", "sl", "pɹ", "bɹ", "tɹ", "dɹ", "kɹ", "gɹ", "fɹ", "θɹ", "ʃɹ",
    "sp", "st", "sk", "sm", "sn", "sw", "sf", "tw", "kw", "dw", "gw", "θw",
    "pj", "bj", "kj", "gj", "mj", "nj", "fj", "vj", "hj", "lj",
    "spl", "spɹ", "stɹ", "skɹ", "skw", "skj", "spj",
];

/// One syllable of a word, before it gets its text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyllableParts {
    pub onset: Vec<Segment>,
    pub nucleus: Segment,
    pub coda: Vec<Segment>,
}

/// Split one word's segments into onset / nucleus / coda syllables.
pub fn syllabify_segments(segs: &[Segment]) -> Vec<SyllableParts> {
    let vi: Vec<usize> = segs.iter().enumerate().filter(|(_, s)| is_vowel(&s.ipa, BodyMix::MALE)).map(|(i, _)| i).collect();
    if vi.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut start = 0;
    for (n, &v) in vi.iter().enumerate() {
        let mut end = segs.len(); // exclusive end of this syllable
        if let Some(&next) = vi.get(n + 1) {
            // consonants between this vowel and the next: the longest legal onset goes right
            let between = &segs[v + 1..next];
            let mut k = 0;
            for len in (1..=3.min(between.len())).rev() {
                let cluster: String = between[between.len() - len..].iter().map(|s| s.ipa.as_str()).collect();
                if ONSETS.contains(&cluster.as_str()) {
                    k = len;
                    break;
                }
            }
            end = next - k;
        }
        out.push(SyllableParts { onset: segs[start..v].to_vec(), nucleus: segs[v].clone(), coda: segs[v + 1..end].to_vec() });
        start = end;
    }
    out
}

// The spelling is indexed in UTF-16 code units, as the JS string was.
fn is_spell_vowel(c: u16) -> bool {
    "aeiouy".bytes().any(|v| v as u16 == c) || is_cyrillic_vowel(c)
}
/// lowercase Cyrillic vowel letters (Russian, Ukrainian, Belarusian), all in the BMP
fn is_cyrillic_vowel(c: u16) -> bool {
    "аеёиоуыэюяіїєў".encode_utf16().any(|v| v == c)
}
fn is_spell_vowel_ci(c: u16) -> bool {
    "aeiouyAEIOUY".bytes().any(|v| v as u16 == c)
}
fn eq_ci(c: u16, lower: u8) -> bool {
    c == lower as u16 || c == lower.to_ascii_uppercase() as u16
}
/// `word.slice(a, b)` (indices clamped to the word)
fn slice(w: &[u16], a: usize, b: usize) -> String {
    let (a, b) = (a.min(w.len()), b.min(w.len()));
    if a >= b { String::new() } else { String::from_utf16_lossy(&w[a..b]) }
}

/// Split a written word into n chunks at its vowel groups (approximate hyphenation).
pub fn split_spelling(word: &str, n: usize) -> Vec<String> {
    if n <= 1 {
        return vec![word.to_string()];
    }
    let w: Vec<u16> = word.encode_utf16().collect();
    let lower: Vec<u16> = word.to_lowercase().encode_utf16().collect();
    // /[aeiouy]+/g on the lowercased word
    let mut groups: Vec<[usize; 2]> = Vec::new();
    let mut i = 0;
    while i < lower.len() {
        if is_spell_vowel(lower[i]) {
            let s = i;
            while i < lower.len() && is_spell_vowel(lower[i]) {
                i += 1;
            }
            groups.push([s, i]);
        } else {
            i += 1;
        }
    }
    let len = w.len();
    // /[^aeiouy]e$/i and /[^aeiouyl]le$/i
    let silent_e = len >= 2 && eq_ci(w[len - 1], b'e') && !is_spell_vowel_ci(w[len - 2]);
    let le = len >= 3 && eq_ci(w[len - 1], b'e') && eq_ci(w[len - 2], b'l') && !is_spell_vowel_ci(w[len - 3]) && !eq_ci(w[len - 3], b'l');
    // a final silent e ("like") is not a syllable, but consonant + "le" is ("twin-kle", "lit-tle")
    if groups.len() > 1 && silent_e && !le && groups[groups.len() - 1][0] + 1 == len {
        groups.pop();
    }
    if groups.len() < n {
        // fewer written vowels than sung syllables (e.g. "fire"): hold the word, then continue marks
        let mut out = vec![word.to_string()];
        out.extend(std::iter::repeat_n("~".to_string(), n - 1));
        return out;
    }
    while groups.len() > n {
        let last = groups.pop().unwrap();
        let k = groups.len() - 1;
        groups[k][1] = last[1];
    }
    let mut cuts: Vec<usize> = vec![0];
    for i in 0..groups.len() - 1 {
        let (a, b) = (groups[i][1], groups[i + 1][0]); // consonant letters a..b between vowel groups
        // a digraph stays together on the right ("any-where", "fa-ther"); otherwise one consonant
        // goes right and longer runs split in the middle ("lit-tle", "won-der")
        let gap = b - a;
        let dig = gap >= 2 && {
            let d = [w.get(b - 2).copied(), w.get(b - 1).copied()];
            ["th", "sh", "ch", "ph", "wh", "ck", "gh"].iter().any(|p| {
                let p = p.as_bytes();
                d[0].is_some_and(|c| eq_ci(c, p[0])) && d[1].is_some_and(|c| eq_ci(c, p[1]))
            })
        };
        cuts.push(if dig { b - 2 } else { a + if gap <= 1 { 0 } else { gap / 2 } });
    }
    cuts.push(len);
    (0..cuts.len() - 1).map(|i| slice(&w, cuts[i], cuts[i + 1])).collect()
}

/// Where lyrics get their pronunciations: `EnglishG2P`, or any `Fn(&str, bool) -> Vec<Segment>`.
pub trait WordSource {
    fn word(&self, w: &str, reduce: bool) -> Vec<Segment>;
}

impl<F: Fn(&str, bool) -> Vec<Segment>> WordSource for F {
    fn word(&self, w: &str, reduce: bool) -> Vec<Segment> {
        self(w, reduce)
    }
}

fn is_lyric_punct(c: char) -> bool {
    matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | '\n')
}
fn is_word_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '\''
}

/// `text.match(/[A-Za-z'][A-Za-z'\-]*|(?<=\s|^)[~-](?=\s|$)|[.,;:!?\n]/g)`, plus IPA words:
/// `[ipa]` or `text[ipa]` (any letters in front, up to the bracket), kept whole as one token, and
/// Cyrillic words.
fn lyric_tokens(text: &str) -> Vec<String> {
    let s: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c == '[' || c.is_alphabetic() || c == '\'' {
            // display text (possibly empty) running straight into a bracket: an IPA word
            let mut j = i;
            while j < s.len() && s[j] != '[' && s[j] != ']' && !js_is_space(s[j]) {
                j += 1;
            }
            if s.get(j) == Some(&'[') {
                let mut k = j + 1;
                while k < s.len() && s[k] != ']' {
                    k += 1;
                }
                out.push(s[i..k.min(s.len())].iter().chain(std::iter::once(&']')).collect());
                i = k + 1;
                continue;
            }
        }
        // a Cyrillic word (Russian), with hyphens and stress marks ("моло+ко", "кто-то")
        if is_cyrillic(c) {
            let start = i;
            i += 1;
            while i < s.len() && (is_cyrillic(s[i]) || s[i] == '-' || s[i] == '\'' || is_stress_mark(s[i])) {
                i += 1;
            }
            out.push(s[start..i].iter().collect());
            continue;
        }
        if is_word_start(c) {
            let start = i;
            i += 1;
            while i < s.len() && (is_word_start(s[i]) || s[i] == '-') {
                i += 1;
            }
            out.push(s[start..i].iter().collect());
            continue;
        }
        let melisma = (c == '~' || c == '-') && (i == 0 || js_is_space(s[i - 1])) && (i + 1 == s.len() || js_is_space(s[i + 1]));
        if melisma || is_lyric_punct(c) {
            out.push(c.to_string());
        }
        i += 1;
    }
    out
}

/// Lyrics text → syllables (needs the pronouncing dictionary for real words).
pub fn lyrics_to_syllables(text: &str, g2p: &impl WordSource) -> Vec<Syllable> {
    let mut out: Vec<Syllable> = Vec::new();
    // words, melisma marks and phrase punctuation, in order
    let tokens = lyric_tokens(&text.replace('’', "'"));
    // a vowelless Russian preposition ("в", "с", "к") waiting to join the next word
    let mut pre: Option<String> = None;
    for tok in &tokens {
        // (a Cyrillic word with IPA in brackets is an IPA word, below)
        if has_cyrillic(tok) && !tok.contains('[') {
            let bare: String = tok.chars().filter(|c| !is_stress_mark(*c)).collect();
            if !bare.chars().flat_map(char::to_lowercase).any(russian::is_vowel_letter) && bare.chars().count() <= 2 {
                pre = Some(match pre.take() {
                    Some(p) => format!("{p}{JOIN}{tok}"),
                    None => tok.clone(),
                });
                continue;
            }
            // a hyphenated pair of words is two words ("чижик-пыжик"); a clitic stays on its
            // word ("кто-то", "из-за")
            let words: Vec<String> = if russian::is_clitic_compound(&bare) { vec![tok.clone()] } else { tok.split('-').filter(|p| !p.is_empty()).map(str::to_string).collect() };
            for w in words {
                let (word, display) = match pre.take() {
                    Some(p) => (format!("{p}{JOIN}{w}"), format!("{} {w}", p.replace(JOIN, " "))),
                    None => (w.clone(), w),
                };
                out.append(&mut russian_syllables(&word, &display, g2p));
            }
            continue;
        }
        // a preposition left with no word after it has nothing to sing
        pre = None;
        if tok.chars().count() == 1 && tok.chars().all(is_lyric_punct) {
            if let Some(last) = out.last_mut() {
                last.phrase_end = true;
            }
            continue;
        }
        if tok == "-" || tok == "~" {
            if let Some(prev) = out.last() {
                let s = Syllable { text: "~".into(), onset: vec![], nucleus: prev.nucleus.clone(), coda: vec![], melisma: Some(true), word_end: prev.word_end, phrase_end: false };
                out.push(s);
            }
            continue;
        }
        if let Some((display, ipa)) = ipa_token(tok) {
            out.append(&mut ipa_syllables(display, ipa));
            continue;
        }
        // typed hyphens split syllables explicitly ("ti-ger"); otherwise the dictionary decides
        let parts: Vec<&str> = tok.split('-').filter(|p| !p.is_empty()).collect();
        let mut syls: Vec<Syllable> = Vec::new();
        for part in &parts {
            let sy = syllabify_segments(&g2p.word(&part.to_lowercase(), false));
            let spell: Vec<String> = if parts.len() > 1 {
                let mut v = vec![part.to_string()];
                v.extend(std::iter::repeat_n("~".to_string(), sy.len().saturating_sub(1)));
                v
            } else {
                split_spelling(part, sy.len())
            };
            for (i, s) in sy.into_iter().enumerate() {
                syls.push(Syllable { text: spell.get(i).cloned().unwrap_or_else(|| "~".into()), onset: s.onset, nucleus: s.nucleus, coda: s.coda, melisma: None, word_end: false, phrase_end: false });
            }
        }
        if let Some(last) = syls.last_mut() {
            last.word_end = true;
        }
        out.append(&mut syls);
    }
    out
}

/// `text[ipa]` → ("text", "ipa")
fn ipa_token(tok: &str) -> Option<(&str, &str)> {
    let (display, rest) = tok.split_once('[')?;
    Some((display, rest.strip_suffix(']')?))
}

/// One word's segments → open syllables, as sung: the consonants between two vowels open the next
/// syllable, but a sonorant (/j/, a nasal, a liquid) before another consonant closes the one
/// before ("voj.na", "fɐn.tan.kʲe", "zʲem.lʲa"; "ma.jor", "pʲe.snʲa"); `breaks[k]`: an explicit
/// break after segment k.
fn open_syllables(segs: &[Segment], breaks: &[bool]) -> Vec<SyllableParts> {
    let vi: Vec<usize> = segs.iter().enumerate().filter(|(_, s)| is_vowel(&s.ipa, BodyMix::MALE)).map(|(i, _)| i).collect();
    let mut out = Vec::with_capacity(vi.len());
    let mut start = 0;
    for (n, &v) in vi.iter().enumerate() {
        let end = match vi.get(n + 1) {
            Some(&next) => match (v..next).find(|&k| breaks.get(k).copied().unwrap_or(false)) {
                Some(k) => k + 1,
                None => {
                    let mut k = v + 1;
                    while k + 1 < next && is_sonorant(&segs[k].ipa) {
                        k += 1;
                    }
                    k
                }
            },
            None => segs.len(),
        };
        out.push(SyllableParts { onset: segs[start..v].to_vec(), nucleus: segs[v].clone(), coda: segs[v + 1..end].to_vec() });
        start = end;
    }
    out
}

fn is_sonorant(ipa: &str) -> bool {
    matches!(ipa.trim_end_matches('ʲ'), "j" | "m" | "n" | "ŋ" | "ɲ" | "l" | "ɫ" | "r" | "ɾ" | "ɹ")
}

/// One Russian word (perhaps with a joined preposition, "в|лесу") → sung syllables: the G2P's
/// segments (unreduced: sung), open syllables, the spelling split to match for the notes.
fn russian_syllables(word: &str, display: &str, g2p: &impl WordSource) -> Vec<Syllable> {
    let segs = g2p.word(&word.to_lowercase(), false);
    let parts = open_syllables(&segs, &[]);
    let n = parts.len();
    let shown: String = display.chars().filter(|c| !is_stress_mark(*c)).collect();
    let texts = russian::split_spelling(&shown, n).unwrap_or_else(|| split_spelling(&shown, n));
    parts
        .into_iter()
        .zip(texts)
        .enumerate()
        .map(|(k, (p, text))| Syllable { text, onset: p.onset, nucleus: p.nucleus, coda: p.coda, melisma: None, word_end: k + 1 == n, phrase_end: false })
        .collect()
}

/// One bracketed IPA lyric → syllables (each IPA word ends a word). Dots and hyphens are explicit
/// syllable breaks; elsewhere consonants between vowels open the next syllable, but a /j/ that
/// precedes another consonant closes the one before ("voj.na", "ma.jor").
fn ipa_syllables(display: &str, ipa: &str) -> Vec<Syllable> {
    let mut parts: Vec<(SyllableParts, bool)> = Vec::new();
    for word in ipa.split(js_is_space).filter(|w| !w.is_empty()) {
        // the tokenizer marks a "." with word_end on the segment before it: that is our break
        let mut segs: Vec<Segment> = tokenize_ipa(&word.replace('-', ".")).into_iter().filter(|s| s.ipa != "_").collect();
        let breaks: Vec<bool> = segs.iter().map(|s| s.word_end == Some(true)).collect();
        for s in segs.iter_mut() {
            s.word_end = None;
        }
        let sy = open_syllables(&segs, &breaks);
        let n = sy.len();
        parts.extend(sy.into_iter().enumerate().map(|(k, p)| (p, k + 1 == n)));
    }
    let n = parts.len();
    // the text on the notes: the given spelling split to match, or the IPA itself
    let display_parts: Vec<&str> = display.split('-').filter(|p| !p.is_empty()).collect();
    let texts: Vec<String> = if display.is_empty() {
        parts.iter().map(|(p, _)| p.onset.iter().chain(std::iter::once(&p.nucleus)).chain(p.coda.iter()).map(|s| s.ipa.as_str()).collect()).collect()
    } else if display_parts.len() == n {
        display_parts.iter().map(|p| p.to_string()).collect()
    } else {
        split_spelling(&display_parts.concat(), n)
    };
    parts
        .into_iter()
        .zip(texts)
        .map(|((p, word_end), text)| Syllable { text, onset: p.onset, nucleus: p.nucleus, coda: p.coda, melisma: None, word_end, phrase_end: false })
        .collect()
}
