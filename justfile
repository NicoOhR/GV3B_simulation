run:
  bspc rule -a "three_body" desktop=^3 state=floating
  cargo run &
  bspc desktop -f ^3
  bspc rule -r "three_body"

