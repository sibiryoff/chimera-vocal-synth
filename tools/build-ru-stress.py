#!/usr/bin/env python3
"""Build public/data/ru-stress.txt.gz: Russian word stress for the Russian G2P.

Sources:
  OpenRussian (https://github.com/Badestrand/russian-dictionary, CC BY-SA 4.0): nouns.csv,
      verbs.csv, adjectives.csv, others.csv: ~55k hand-checked lemmas with their forms, the
      stress marked by ' after the vowel, ё written. Preferred wherever it has the form.
  Koziev/NLP_Datasets (https://github.com/Koziev/NLP_Datasets, CC0 1.0):
      Stress/all_accents.zip   word forms with the stressed vowel marked (^ before it), spread
                               mechanically over each lemma's forms: wide, but wrong on many
                               common forms (парк^е, уж^ин, м^инут), so only the fallback
      WordformFrequencies/Data/term2freq.7z   word-form frequencies in an ~8 GB corpus

Output: one line per word form, "spelling<TAB>n", n the stressed vowel's 1-based position among
the word's vowels. A form with two stresses (мо́ря / моря́, доро́га / дорога́) keeps one: a function
word's first; else the reading of the lemma the corpus uses most (counted on the forms only that
lemma spells), short adjectives and comparatives far behind full forms; within a lemma the first
in the order singular, plural, with the locative in -у́ before the dative.
The Koziev list writes no ё at all (ещ^е, ид^ет, зел^еный), so there a stressed е becomes ё when
the corpus spells the form with ё in at least 1% of its uses (and 3 times). Only words of two or
more syllables, the N most frequent. Sorted, gzipped with mtime 0 so the file is reproducible.

  python3 tools/build-ru-stress.py all_accents.tsv term2freq.dat openrussian-dir [N=400000]
"""
import collections
import csv
import gzip
import re
import sys

DEBUG = False
# a dictionary form (бе́лка) against another lemma's oblique form (белка́, of бело́к)
HEAD = 3.0
VOWELS = set("аеёиоуыэюя")

# Frequent forms the source gets wrong (a homograph from another lemma won: у́же "narrower" over
# уже́ "already") or lacks, from proofreading the 500 most frequent: spelling, stressed vowel
OVERRIDES = {
    "ещё": 2, "уже": 2, "после": 1, "потом": 2, "него": 2, "того": 2, "пока": 2, "себе": 2,
    "моя": 2, "мало": 1, "перед": 1, "причём": 2, "вперёд": 2, "дома": 1, "кругом": 2,
    "смотрит": 1, "говорю": 3, "увидев": 2, "услышав": 2, "росла": 2, "пришла": 2, "дела": 2,
    # the plural is the common reading (ру́ки, not the genitive руки́), and locatives OpenRussian lacks
    "руки": 1, "ноги": 1, "глаза": 2, "берегу": 3, "краю": 2, "снегу": 2,
    # OpenRussian's list of adverbs and function words, read first, has a rare reading or an error
    "семью": 2, "господа": 3, "пошло": 2, "полагается": 3, "вольно": 1, "охотничье": 2, "вини": 2,
    # from proofreading the 350 most frequent forms whose stress OpenRussian changed: a rarer lemma
    # or form won (ко́му of кома, во́рота of ворот, пола́, горе́, о́блака), or its variant is not
    # the standard one (на́чался, уда́лось, происходи́т)
    "самое": 1, "самом": 1, "самого": 1, "самой": 1, "самому": 1, "кому": 2, "никого": 3, "никому": 3,
    "происходит": 3, "поддерживать": 2, "начался": 3, "принялся": 3, "удалось": 3, "собрались": 3,
    "войска": 2, "ворота": 2, "ворот": 2, "полчаса": 3, "пола": 1, "горе": 1, "таки": 1, "облака": 3,
    "губы": 1, "слёзы": 1, "звёзды": 1, "яйца": 1, "спали": 1, "соли": 1, "призы": 2, "костей": 2,
    "мастерской": 3, "аэропорту": 5, "занялся": 3, "взялись": 2, "собралось": 3,
}
# украи́нский (OpenRussian keeps the older укра́инский)
OVERRIDES.update({"украинск" + e: 3 for e in ("ий", "ого", "ому", "им", "ом", "ая", "ой", "ую", "ое", "ие", "их", "ими")})


def openrussian(folder: str, freq: collections.Counter) -> dict[str, tuple[str, int]]:
    """form (ё as е) → (spelling, stressed vowel), one reading of each form."""
    # (lemma, key, spelling, stressed vowel, weight of the slot), in reading order
    found: list[tuple[int, str, str, int, float]] = []
    function_words: dict[str, tuple[str, int]] = {}
    lemma = 0

    def add(cell: str, slot: float = 1.0, into: dict | None = None) -> None:
        for form in re.split(r"[,;/]", cell):
            form = form.strip().lower()
            if not form or not re.fullmatch(r"[а-яё'-]+", form):
                continue
            plain = form.replace("'", "")
            vowels = [i for i, c in enumerate(plain) if c in VOWELS]
            if len(vowels) < 2:
                continue
            k = form.find("'")
            if k > 0 and form.count("'") == 1 and form[k - 1] in VOWELS:
                at = k - 1
            elif "'" not in form and plain.count("ё") == 1:
                at = plain.index("ё")
            else:
                continue
            key, n = plain.replace("ё", "е"), vowels.index(at) + 1
            if into is not None:
                into.setdefault(key, (plain, n))
            else:
                found.append((lemma, key, plain, n, slot))

    def rows(name: str):
        with open(f"{folder}/{name}.csv", encoding="utf-8", newline="") as f:
            yield from csv.DictReader(f, delimiter="\t", quoting=csv.QUOTE_NONE)

    for r in rows("others"):
        add(r["accented"] or "", into=function_words)
    for r in rows("nouns"):
        lemma += 1
        add(r["accented"] or "", HEAD)
        # the locative in stressed -у (в лесу́, в году́, на берегу́) is spelled as the dative
        # (ле́су, го́ду) and far more common than it
        for form in re.split(r"[,;/]", r.get("sg_prep") or ""):
            if form.strip().endswith(("у'", "ю'")):
                add(form)
        for k in ("sg_nom", "sg_gen", "sg_dat", "sg_acc", "sg_inst", "sg_prep", "pl_nom", "pl_gen", "pl_dat", "pl_acc", "pl_inst", "pl_prep"):
            add(r.get(k) or "")
    for r in rows("adjectives"):
        lemma += 1
        add(r["accented"] or "", HEAD)
        for k in r:
            if k and k.startswith("decl_"):
                add(r.get(k) or "")
        # short forms and comparatives are rare next to a noun or a full form spelled the same
        for k in ("short_m", "short_f", "short_n", "short_pl", "comparative", "superlative"):
            add(r.get(k) or "", 0.05)
    for r in rows("verbs"):
        lemma += 1
        add(r["accented"] or "", HEAD)
        for k in r:
            if k and (k.startswith("presfut_") or k.startswith("past_") or k.startswith("imperative_")):
                add(r.get(k) or "")
    # a lemma weighs what the corpus has of the forms only it spells: пиро́г (пирогом, пирогов)
    # over пиро́га "pirogue" (пирогой) for the forms they share (пирога, пироги, пирогами)
    owners: dict[str, set[int]] = collections.defaultdict(set)
    for lem, key, _, _, _ in found:
        owners[key].add(lem)
    weight: collections.Counter = collections.Counter()
    for key, ls in owners.items():
        if len(ls) == 1:
            weight[next(iter(ls))] += freq.get(key, 0)
    best: dict[str, tuple[float, str, int]] = {}
    for lem, key, plain, n, slot in found:
        score = (weight[lem] + 1) * slot
        if key not in best or score > best[key][0]:
            best[key] = (score, plain, n)
    out = {key: (plain, n) for key, (_, plain, n) in best.items()}
    if DEBUG:
        clash = sorted((k for k in function_words if k in out and out[k][1] != function_words[k][1]), key=lambda k: -freq.get(k, 0))
        print("function word vs lemma:", " ".join(f"{k}:{function_words[k][1]}/{out[k][1]}" for k in clash[:150]), file=sys.stderr)
    out.update(function_words)
    return out


def main() -> None:
    accents, freqs, orus = sys.argv[1], sys.argv[2], sys.argv[3]
    n_keep = int(sys.argv[4]) if len(sys.argv) > 4 else 400_000
    freq: collections.Counter = collections.Counter()
    with open(freqs, encoding="utf-8") as f:
        for line in f:
            p = line.rstrip("\n").split("\t")
            if len(p) >= 3 and p[2].isdigit():
                freq[p[0].lower()] += int(p[2])
    both = collections.Counter()
    for w, c in freq.items():
        both[w.replace("ё", "е")] += c
    entries: dict[str, tuple[str, int]] = {}
    with open(accents, encoding="utf-8") as f:
        for line in f:
            p = line.rstrip("\n").split("\t")
            if len(p) < 2:
                continue
            word, marked = p[0].lower(), p[1].lower()
            if not word or not all(c.isalpha() or c == "-" for c in word):
                continue
            k = marked.find("^")
            plain = marked.replace("^", "")
            vowels = [i for i, c in enumerate(plain) if c in VOWELS]
            if len(vowels) < 2 or k < 0 or k not in vowels:
                continue
            key = plain.replace("ё", "е")
            # the first entry wins (the source keeps one stress per form)
            entries.setdefault(key, (plain, vowels.index(k) + 1))
    # term2freq counts е and ё spellings apart; a form's frequency is both
    for key, (plain, n) in entries.items():
        k = [i for i, c in enumerate(plain) if c in VOWELS][n - 1]
        if plain[k] == "е":
            yo = plain[:k] + "ё" + plain[k + 1 :]
            fy, fe = freq.get(yo, 0), freq.get(plain, 0)
            if fy >= 3 and fy >= 0.01 * (fe + fy):
                entries[key] = (yo, n)
    checked = openrussian(orus, both)
    differ = sum(both.get(k, 0) for k, v in checked.items() if k in entries and entries[k][1] != v[1])
    known = sum(both.get(k, 0) for k in checked if k in entries)
    print(f"OpenRussian: {len(checked)} forms; of the corpus uses both lists cover, {100 * differ / max(1, known):.1f}% had another stress", file=sys.stderr)
    entries.update(checked)
    keep = set(sorted(entries, key=lambda k: (-both.get(k, 0), k))[:n_keep])
    for w, n in OVERRIDES.items():
        key = w.replace("ё", "е")
        entries[key] = (w, n)
        keep.add(key)
    text = "".join(f"{entries[k][0]}\t{entries[k][1]}\n" for k in sorted(keep))
    out = "public/data/ru-stress.txt.gz"
    with open(out, "wb") as raw, gzip.GzipFile(fileobj=raw, mode="wb", compresslevel=9, mtime=0, filename="") as gz:
        gz.write(text.encode("utf-8"))
    print(f"{out}: {len(keep)} forms of {len(entries)} ({len(OVERRIDES)} overridden)")


if __name__ == "__main__":
    main()
