#!/usr/bin/env python3
"""Build public/data/ru-stress.txt.gz: Russian word stress for the Russian G2P.

Sources, both from Koziev/NLP_Datasets (https://github.com/Koziev/NLP_Datasets, CC0 1.0):
  Stress/all_accents.zip              word forms with the stressed vowel marked (^ before it),
                                      collected from Wikipedia and Wiktionary and spread over
                                      each lemma's forms (so forms whose stress moves, реки/рекИ,
                                      keep one variant)
  WordformFrequencies/Data/term2freq.7z   word-form frequencies in an ~8 GB corpus

Output: one line per word form, "spelling<TAB>n", n the stressed vowel's 1-based position among
the word's vowels. The stress source writes no ё at all (ещ^е, ид^ет, зел^еный), so a stressed е
becomes ё when the corpus spells the form with ё there in at least 1% of its uses (and 3 times):
ещё 5.5%, берёт 5.3%, идёт 1.5%, осёл 1.3%; небо stays (нёбо 0.2%), so does совершенный (0.9%). Only words of two or more
syllables (one vowel: the stress is plain), the N most frequent. Sorted, gzipped with mtime 0
so the file is reproducible.

  python3 tools/build-ru-stress.py all_accents.tsv term2freq.dat [N=400000]
"""
import collections
import gzip
import sys

VOWELS = set("аеёиоуыэюя")

# Frequent forms the source gets wrong (a homograph from another lemma won: у́же "narrower" over
# уже́ "already") or lacks, from proofreading the 500 most frequent: spelling, stressed vowel
OVERRIDES = {
    "ещё": 2, "уже": 2, "после": 1, "потом": 2, "него": 2, "того": 2, "пока": 2, "себе": 2,
    "моя": 2, "мало": 1, "перед": 1, "причём": 2, "вперёд": 2, "дома": 1, "кругом": 2,
    "смотрит": 1, "говорю": 3, "увидев": 2, "услышав": 2, "росла": 2, "пришла": 2, "дела": 2,
}


def main() -> None:
    accents, freqs = sys.argv[1], sys.argv[2]
    n_keep = int(sys.argv[3]) if len(sys.argv) > 3 else 400_000
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
