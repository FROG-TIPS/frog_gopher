# FROG.TIPS GOPHER SERVER

```
               _     __,..---""-._                 ';-,        A GOPHER
        ,    _/_),-"`             '-.                `\\       IS BASICALLY
       \|.-"`    -_)                 '.                ||      A MOUSE,
       /`   a   ,                      \              .'/      RIGHT???
       '.___,__/                 .-'    \_        _.-'.'       
          |\  \      \         /`        _`""""""`_.-'         FOLKS, DO YOU
             _/;--._, &gt;        |   --.__/ `""""""`          KNOW HOW HARD
           (((-'  __//`'-......-;\      )                      IT IS TO FIND
                (((-'       __//  '--. /                       GOPHER ASCII ART
        jgs               (((-'    __//                        IN 2017
                                 (((-'                         
```

[![Build Status](https://travis-ci.org/FROG-TIPS/frog_gopher.svg?branch=master)](https://travis-ci.org/FROG-TIPS/frog_gopher)

WELCOME, FRIEND TO THE FROG.TIPS GOPHER SERVER. LOVINGLY HAND-CRAFTED IN RUST,
THIS SERVER IS INTENDED TO DELIGHT AND AMAZE.

VIEW IT FOR YOURSELF AT [GOPHER://GOPHER.FROG.TIPS](http://gopher.floodgap.com/gopher/gw.lite?gopher://gopher.FROG.TIPS)

## BUILDING THIS BAD BOY

YOU WILL NEED TO BUILD OFF NIGHTLY WITH `rustc stable`. INSTALL RUST VIA [RUSTUP](https://www.rustup.rs/).

THEN:
```
cargo run -- $HOST:$PORT --ext_addr $HOST:$PORT --api_key $FROG_TIPS_API_KEY
```

IF YOU DO NOT HAVE A FROG.TIPS API KEY, DON'T WORRY: FROG WILL FIND YOU ONE.
