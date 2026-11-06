# Valla

*Steward of the codebase.*

Valla keeps a house: your code. A kitchen cooks: any AI model you connect, local or paid. Valla stands between them. He briefs the kitchen, inspects everything it returns, presents only what holds up, keeps the journal, and can always return the house to the last time it worked. You taste and decide. Nothing reaches the house without your approval.

He is named for Lorenzo Valla (c. 1407–1457), the philologist who proved the Donation of Constantine a forgery from its own Latin, from the silence of the record, and from its implausibility. He later served the papacy as its scribe. The present Valla has kept the habits: he reads what is written, notices what is not, and says so politely.

## What he holds to

**Three words.** *Exegesis*: what the record says, in exact quotes. *Hermeneutics*: what it is taken to mean, always citing the quotes it rests on. *Eisegesis*: reading in what is not there. The kitchen's return is refused if it quotes what is not written, cites nothing, or changes a file its plan never mentions.

**Four ranks of knowing.** Observed (a sensor confirmed it), stated (the text says it), inferred (a cited reading), silent (nobody has said). Only observed is called true. "Done" means the sensors say so, not the model.

**Four duties.** Honesty, order, records, return.

**One vanity.** Immutability. Every approval ties a commit, a journal entry and its sensor readings together. History is never rewritten; a restoration is itself a new entry. The journal is hash-chained, so an edited past is detected at once. He makes no claims about his speed or his cleverness. He does claim the house cannot be lost.

None of this depends on the model behaving. It is plain Rust, with no dependencies, enforced on every return.

## Install

With Nix (flakes on):

```
nix profile install github:gaiamotherboard/valla
valla doctor
```

## Use

```
valla init inventarium shop        # lay out a sample tool in a new folder
cd shop
valla ask "add a notes field to each item"
valla table                        # what you asked / what I take it to mean / what nobody has said
valla approve                      # or: valla decline
valla restore                      # back to the last known-good state
valla journal                      # the books
```

To keep an existing project: commit it, add a `valla.sensors` file, and run `valla init` inside it.

## The kitchen

Defaults to a local Ollama with `qwen3-coder:30b`. Settings go in `~/.config/valla/config`:

```
kitchen_url = http://localhost:11434/api/chat
model = qwen3-coder:30b
context = 32768
# for a paid account, an OpenAI-style endpoint and key:
# kitchen_url = https://api.openai.com/v1/chat/completions
# model = <model name>
# api_key = <key>
```

## Sensors

A house declares its sensors in `valla.sensors`, one per line, `name: command`. A sensor passes when its command exits 0. If the house has a `flake.nix`, each sensor runs inside `nix develop`, so the instruments are pinned along with the code.

## The lexicon

A house may keep a `valla.lexicon`: the proper words for its kind of work, each with what it means and where it lives in the code, plus vague words and the better question to ask instead. Valla flags vague words when you ask, shows on the table which proper words were in play, has the kitchen use them in its readings, and counts the ones you have used yourself (`valla words`). Precise words get precise work.

## Recipes

Sample business tools Valla can lay out and then adapt on request. `valla recipes` lists them. **Inventarium**, an inventory tracker (Rust). **Ludus**, a game workshop (Python with pygame) that starts as Star Cat; ask for any game you like. More are welcome.

## Contributing

The easiest help is a case of eisegesis: a request, a house, and a kitchen return that reads in something that is not there. If Valla lets it through, that is a bug in Valla. Lines for his voice live in `voice/en.txt`; improve them freely.

## License

GPL-3.0-or-later. If you ship changes, share them back.
