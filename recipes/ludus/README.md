# Ludus

A game workshop. It starts as **Star Cat**: a cat runs along the bottom and catches falling stars. Miss three and the game is over. Press R to play again.

It is written in Python with pygame. You don't have to write any of it yourself. Ask Valla for what you want, look at what he puts on the table, and approve it if you like it.

## Play

```
nix develop --command python3 game.py
```

Left and right arrows (or A and D) to move.

## Learn the words

Making games has its own language: *variable*, *function*, *collision*, *game loop*. The better your words, the better Valla's work. `QUESTS.md` is a ladder of twelve changes, each teaching a few words. `valla words` lists them all, with where each lives in the code, and marks the ones you have used.

If you ask with a vague word like *faster* or *better*, Valla will say what it leaves unsaid and suggest the precise way to ask.

## Make it yours

From inside this folder:

```
valla ask "make the stars fall faster as the score goes up"
valla table
valla approve
```

Ideas to ask for:

- a high score that is remembered between games
- bad stars that cost a life if you catch them
- a jump, and platforms to jump on
- sound when you catch a star
- a title screen
- a second player on W and S keys
- a different hero: a robot, a fish, a spaceship

Be specific. If you say "make it harder," Valla will tell you the record is silent on *how*, and ask.

If a change breaks the game, nothing is lost:

```
valla restore
```

## How the game is put together

- `world.py` holds the rules: who moves, what scores, when the game ends. No drawing.
- `game.py` draws the world and reads the keys.
- `test_world.py` checks the rules.

## What the sensors check

`valla.sensors`: the rule tests pass, and the game actually runs for three seconds without crashing. Valla checks both before showing you a change, and again after you approve it.
