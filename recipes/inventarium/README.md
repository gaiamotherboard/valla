# Inventarium

A small inventory tracker. Items have a name, a location and a status (in stock, reserved, sold, retired). One page: add an item, search, change a status. Data lives in one tab-separated file you can open in any spreadsheet.

No dependencies. Everything is in `src/`.

## Run it

```
cargo run -- --port 8080 --data inventarium.tsv --title "My Inventory"
```

Then open http://localhost:8080

## Change it with Valla

From inside this folder:

```
valla ask "add a notes field to each item"
valla table
valla approve
```

Things owners commonly ask for:

- a notes or serial-number field
- a price, and a total value at the top
- export to CSV
- a printable label for each item
- different statuses for their business

## What the sensors check

`valla.sensors` lists them: it builds, its tests pass, and it serves a page. Valla runs them before anything is put on your table and again after you approve.

## Where the data is

`inventarium.tsv` (or whatever `--data` names). It is ignored by git, so your records are never part of a code change, and a `valla restore` never touches them.
