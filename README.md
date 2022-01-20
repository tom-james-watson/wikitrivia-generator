# WikiTrivia Generator

The code for generating cards for https://wikitrivia.tomjwatson.com.

The repository for the website can be found [here](https://github.com/tom-james-watson/wikitrivia).

## Development

### Requirements

```
sudo apt install pbzip2 jq
sudo npm i -g wikibase-dump-filter
```

### Running

```
cargo run
```

## Notes

### Important properties

P31 : instance of
P18 : image

### Date properties

P580 : start time
P582 : end time
P1249 : time of earliest written record
P575 : time of discovery or invention
P569 : date of birth
P570 : date of death
P571 : inception
P576 : dissolved, abolished or demolished date
P577 : publication date
P1191 : date of first performance
P1319 : earliest date
P1326 : latest date
P1619 : date of official opening
P6949 : announcement date
P7124 : date of the first one
P7589 : date of assent
P8556 : extinction date

### Processing

Get all items with date claims and keep a simplified version of the item:

```
pbzip2 -d latest-all.json.bz2 -c | wikibase-dump-filter --claim 'P580|P582|P1249|P575|P570|P571|P576|P577|P1191|P1319|P1326|P1619|P6949|P7124|P7589|P8556' --simplify > processed.json
```

```
pbzip2 -d latest-all.json.bz2 -c | wikibase-dump-filter --claim 'P1249|P575|P577' --simplify > processed2.json
```

### Ranking

Rank them by fetching `contentlength` header of english wikipedia entry: `curl -I https://en.wikipedia.org/wiki/Paris`.
