#!/bin/bash

cd XiangShan/build

# Patch the clock gating file
FILE="STD_CLKGT_func.v"
sed -i 's/assign Q = CK & clk_en_reg;/assign Q = CK;/' "$FILE"
echo "Done patching clock gating"
# Patch the SRAM blocks for faster bit gen
i=0

while [ "$i" -le 25 ]; do
    file="array_${i}_ext.v"

    if [ -f "$file" ]; then
        tmp="${file}.tmp"
        changed=0

        while IFS= read -r line || [ -n "$line" ]; do
            values=$(printf '%s\n' "$line" |
                sed -n 's/^[[:space:]]*reg[[:space:]]\+\[\([0-9][0-9]*\):0\][[:space:]]\+ram[[:space:]]\+\[\([0-9][0-9]*\):0\][[:space:]]*;.*/\1 \2/p')

            if [ -n "$values" ]; then
                set -- $values
                X=$1
                Y=$2

                if [ "$((X * Y))" -ge 2560 ]; then
                    line=$(printf '%s\n' "$line" |
                        sed 's/^\([[:space:]]*\)/\1(* ram_style="ultra" *) /')
                    changed=1
                fi
            fi

            printf '%s\n' "$line"
        done < "$file" > "$tmp"

        mv "$tmp" "$file"

        if [ "$changed" -eq 1 ]; then
            echo "Changed: $file"
        fi
    fi

    i=$((i + 1))
done

cd ../../
vivado -mode batch -source xiangshan-fpga.tcl
