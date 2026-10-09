#!/bin/sh
# assembles src/gui/index.html from src/gui/parts (the page is served from one file)
cd "$(dirname "$0")/src/gui" && cat parts/1_head.html parts/2_core.js parts/3_main.js parts/4_formulas_light.js parts/5_post_navi.js parts/6_tools.js parts/7_extra.js parts/8_start.js > index.html
