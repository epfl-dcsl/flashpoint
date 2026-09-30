# set_u55c_vio_one.tcl
#
# The FPGA must already be programmed with the matching design.

set HW_SERVER_URL "localhost:3121"
set HW_TARGET_PATH \
    "localhost:3121/xilinx_tcf/Xilinx/XFL1Y1Y0BEQVA"

set DEVICE_NAME "xcu280_u55c_0"

set LTX_FILE \
    "./project_X/project_X.runs/impl_1/u55c_top.ltx"

set VIO_CELL_NAME \
    "PCIE_HBM_SUB_SYS_i/vio_0"

set VIO_PROBE_NAME \
    "PCIE_HBM_SUB_SYS_i/vio_0_probe_out0"

set exit_code 0

if {[catch {
    puts "Opening Hardware Manager..."
    open_hw_manager

    puts "Connecting to $HW_SERVER_URL..."
    connect_hw_server -url $HW_SERVER_URL

    # Select the exact JTAG target.
    set hw_target [get_hw_targets -quiet $HW_TARGET_PATH]

    if {[llength $hw_target] != 1} {
        puts "Available hardware targets:"
        foreach target [get_hw_targets -quiet] {
            puts "  $target"
        }

        error "Could not uniquely find target: $HW_TARGET_PATH"
    }

    current_hw_target $hw_target

    puts "Opening target $HW_TARGET_PATH..."
    open_hw_target

    # Select the exact FPGA.
    set hw_device [get_hw_devices -quiet $DEVICE_NAME]

    if {[llength $hw_device] != 1} {
        puts "Available hardware devices:"
        foreach device [get_hw_devices -quiet] {
            puts "  $device"
        }

        error "Could not uniquely find device: $DEVICE_NAME"
    }

    current_hw_device $hw_device

    if {![file exists $LTX_FILE]} {
        error "LTX file does not exist: $LTX_FILE"
    }

    # Load the debug-probe definitions into this Vivado session.
    puts "Loading LTX file..."
    set_property PROBES.FILE      $LTX_FILE $hw_device
    set_property FULL_PROBES.FILE $LTX_FILE $hw_device

    puts "Refreshing hardware device..."
    refresh_hw_device $hw_device

    # Find the exact VIO instance.
    set hw_vio [get_hw_vios \
        -quiet \
        -of_objects $hw_device \
        -filter "CELL_NAME=~\"$VIO_CELL_NAME\""]

    if {[llength $hw_vio] != 1} {
        puts "Available VIO cores:"
        foreach vio [get_hw_vios -quiet -of_objects $hw_device] {
            puts "  $vio, CELL_NAME=[get_property CELL_NAME $vio]"
        }

        error "Could not uniquely find VIO core: $VIO_CELL_NAME"
    }

    # Find the exact output probe.
    set hw_probe [get_hw_probes \
        -quiet \
        $VIO_PROBE_NAME \
        -of_objects $hw_vio]

    if {[llength $hw_probe] != 1} {
        puts "Available probes on $VIO_CELL_NAME:"
        foreach probe [get_hw_probes -quiet -of_objects $hw_vio] {
            puts "  $probe"
        }

        error "Could not uniquely find VIO probe: $VIO_PROBE_NAME"
    }

    # Drive the VIO output high.
    puts "Setting $VIO_PROBE_NAME to 1..."

    set_property OUTPUT_VALUE 1 $hw_probe
    commit_hw_vio $hw_probe

    # Read the actual output value back from the VIO core.
    refresh_hw_vio -update_output_values 1 $hw_vio

    set actual_value [get_property OUTPUT_VALUE $hw_probe]

    puts "VIO output readback: $actual_value"

    if {$actual_value ne "1"} {
        error "VIO output readback was '$actual_value'; expected '1'"
    }

    puts "SUCCESS: $VIO_PROBE_NAME is set to 1."

} error_message error_options]} {
    puts stderr ""
    puts stderr "ERROR: $error_message"

    if {[dict exists $error_options -errorinfo]} {
        puts stderr [dict get $error_options -errorinfo]
    }

    set exit_code 1
}

# Closing Vivado's connection does not undo the committed VIO value.
catch {close_hw_target}
catch {disconnect_hw_server}
catch {close_hw_manager}

exit $exit_code
