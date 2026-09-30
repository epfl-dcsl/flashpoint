# program_u55c.tcl

open_hw_manager
connect_hw_server -url localhost:3121
open_hw_target

set device [get_hw_devices xcu280_u55c_0]

if {[llength $device] != 1} {
    puts stderr "ERROR: Could not uniquely find xcu280_u55c_0"
    puts stderr "Detected devices: [get_hw_devices]"
    exit 1
}

current_hw_device $device

set_property PROBES.FILE \
    {./project_X/project_X.runs/impl_1/u55c_top.ltx} \
    $device

set_property FULL_PROBES.FILE \
    {./project_X/project_X.runs/impl_1/u55c_top.ltx} \
    $device

set_property PROGRAM.FILE \
    {./project_X/project_X.runs/impl_1/u55c_top.bit} \
    $device

puts "Programming xcu280_u55c_0..."
program_hw_devices $device

refresh_hw_device $device

puts "Successfully programmed xcu280_u55c_0."

close_hw_target
disconnect_hw_server
close_hw_manager
exit
