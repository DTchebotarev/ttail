#!/usr/bin/env bash
# Generates colored log output to test ANSI color preservation in ttail.
# The color set on line 1 (bold red) should persist even after it scrolls
# out of the buffer. Watch for color bleeding or loss as lines scroll.
#
# Usage: ./scripts/dev/gen_colored_logs.sh | cargo run
#
# Expected behavior:
#   - Lines 1-5: bold red (set explicitly on line 1, no reset)
#   - Lines 6-10: green (reset then green on line 6)
#   - Lines 11-15: blue background with white text (set on line 11)
#   - Lines 16-20: 256-color orange (set on line 16)
#   - Lines 21-25: RGB purple (set on line 21)
#   - Lines 26-30: mixed — each line sets its own color
#   - Lines 31-40: rapid color changes to stress test
#
# Since ttail shows 10 lines at a time, by line 12 the originating
# color line (line 1) has scrolled out. If colors are preserved
# correctly, lines should still render in the active color.
set -euo pipefail

RED='\x1B[1;31m'
GREEN='\x1B[32m'
BLUE_BG='\x1B[44;37m'
ORANGE_256='\x1B[38;5;208m'
PURPLE_RGB='\x1B[38;2;128;0;255m'
RESET='\x1B[0m'
BOLD='\x1B[1m'
DIM='\x1B[2m'
ITALIC='\x1B[3m'
UNDERLINE='\x1B[4m'

delay() { sleep 0.05; }

# Phase 1: Bold red, no reset between lines
printf "${RED}[Phase 1] Bold red starts here\n"; delay
for i in $(seq 2 5); do
  printf "  line $i should still be bold red\n"; delay
done

# Phase 2: Switch to green
printf "${RESET}${GREEN}[Phase 2] Switched to green\n"; delay
for i in $(seq 7 10); do
  printf "  line $i should be green\n"; delay
done

# Phase 3: Blue background, white text
printf "${RESET}${BLUE_BG}[Phase 3] Blue bg, white text\n"; delay
for i in $(seq 12 15); do
  printf "  line $i should have blue background\n"; delay
done

# Phase 4: 256-color orange
printf "${RESET}${ORANGE_256}[Phase 4] 256-color orange\n"; delay
for i in $(seq 17 20); do
  printf "  line $i should be orange\n"; delay
done

# Phase 5: RGB purple
printf "${RESET}${PURPLE_RGB}[Phase 5] RGB purple\n"; delay
for i in $(seq 22 25); do
  printf "  line $i should be purple\n"; delay
done

# Phase 6: Each line has its own color
printf "${RESET}${RED}[Phase 6] This line is red\n"; delay
printf "${GREEN}[Phase 6] This line is green\n"; delay
printf "${BLUE_BG}[Phase 6] This line has blue bg\n"; delay
printf "${RESET}${BOLD}[Phase 6] This line is bold\n"; delay
printf "${RESET}${UNDERLINE}[Phase 6] This line is underlined\n"; delay

# Phase 7: Rapid color changes
printf "${RESET}\n"
for i in $(seq 31 40); do
  code=$((31 + (i % 7)))
  printf "\x1B[${code}m[Phase 7] Rapid color line $i\n"; delay
done

# Final reset
printf "${RESET}"
