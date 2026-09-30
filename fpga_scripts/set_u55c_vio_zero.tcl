# set_u55c_vio_zero.tcl
#
# Standalone Vivado script:
#   - Connects to the exact U55C JTAG target
#   - Loads the matching LTX probe file
#   - Sets PCIE_HBM_SUB_SYS_i/vio_0_probe_out0 to 0
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
    puts "Opening Vivado Hardware Manager..."
    open_hw_manager

    puts "Connecting to hardware server: $HW_SERVER_URL"
    connect_hw_server -url $HW_SERVER_URL

    # Select only the specified U55C JTAG cable.
    set hw_target [get_hw_targets -quiet $HW_TARGET_PATH]

    if {[llength $hw_target] != 1} {
        error "Could not find the hardware target: $HW_TARGET_PATH"
    }

    current_hw_target $hw_target

    puts "Opening hardware target: $HW_TARGET_PATH"
    open_hw_target

    # Select only the specified FPGA.
    set hw_device [get_hw_devices -quiet $DEVICE_NAME]

    if {[llength $hw_device] != 1} {
        puts "Detected hardware devices:"
        foreach device [get_hw_devices -quiet] {
            puts "  $device"
        }

        error "Could not uniquely find device: $DEVICE_NAME"
    }

    current_hw_device $hw_device

    if {![file exists $LTX_FILE]} {
        error "LTX file does not exist: $LTX_FILE"
    }

    # A fresh Vivado session needs the LTX file to discover the
    # ILA/VIO debug cores in the already-programmed design.
    puts "Loading probes file: $LTX_FILE"

    set_property PROBES.FILE      $LTX_FILE $hw_device
    set_property FULL_PROBES.FILE $LTX_FILE $hw_device

    puts "Refreshing device: $DEVICE_NAME"
    refresh_hw_device $hw_device

    # Locate the exact VIO core.
    set hw_vio [get_hw_vios \
        -quiet \
        -of_objects $hw_device \
        -filter "CELL_NAME=~\"$VIO_CELL_NAME\""]

    if {[llength $hw_vio] != 1} {
        puts "Detected VIO cores:"
        foreach vio [get_hw_vios -quiet -of_objects $hw_device] {
            puts "  $vio, CELL_NAME=[get_property CELL_NAME $vio]"
        }

        error "Could not uniquely find VIO core: $VIO_CELL_NAME"
    }

    # Locate the exact VIO output probe.
    set hw_probe [get_hw_probes \
        -quiet \
        $VIO_PROBE_NAME \
        -of_objects $hw_vio]

    if {[llength $hw_probe] != 1} {
        puts "Detected probes for $VIO_CELL_NAME:"
        foreach probe [get_hw_probes -quiet -of_objects $hw_vio] {
            puts "  $probe"
        }

        error "Could not uniquely find VIO probe: $VIO_PROBE_NAME"
    }

    puts "Setting $VIO_PROBE_NAME to 0..."

    set_property OUTPUT_VALUE 0 $hw_probe
    commit_hw_vio $hw_probe

    # Synchronize Vivado's OUTPUT_VALUE property with the actual VIO core.
    refresh_hw_vio -update_output_values $hw_vio

    set actual_value [get_property OUTPUT_VALUE $hw_probe]

    puts "VIO output readback: $actual_value"

    if {$actual_value ne "0"} {
        error "VIO output readback was '$actual_value', expected '0'"
    }

    puts "SUCCESS: $VIO_PROBE_NAME is set to 0."

} error_message error_options]} {
    puts stderr ""
    puts stderr "ERROR: $error_message"

    if {[dict exists $error_options -errorinfo]} {
        puts stderr [dict get $error_options -errorinfo]
    }

    set exit_code 1
}

# Cleanup does not change the committed VIO output value.
catch {close_hw_target}
catch {disconnect_hw_server}
catch {close_hw_manager}

exit $exit_code
