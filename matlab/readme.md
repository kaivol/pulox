# Matlab
Matlab integration uses the class defined in `Pulox.m` and the `mex_pulox` crate. 

To use the `Pulox` class, you need to build this crate and rename the `mex_pulox.dll` to `mex_pulox.mexw64` and place it next to `Pulox.m`.

Build using `cargo build -p mex_pulox`

## Usage
The `Pulox` class represents a connection to a Pulox PPG device. 

### Constructor
Opens the connection with a hardware device.  
You must ensure that a compatible PPG is connected at the given serial port. 

**Arguments**:
1. `callback` (mandatory)  
Callback which gets executed everytime a new measurement is received from the device
The callback is invoked with the following arguments:
   1. Measurement data (4-element uint8 array):  
   `[probe_errors; spo2; pulse_rate; pulse_waveform]`
   2. The Pulox object
2. `port` (optional)  
Name of serial port  
Default: 'COM3'

#### startRealtime 
Instructs the device to start sending real time data.  
You need to make sure that the device is turned on and ready to start sending real time data.


#### stopRealtime Stop real time data
Instructs the device to stop sending real time data.  
Note that the callback might still receive new measurements after the call to this method.

## Example
For an example, look at `pulox_demo.m`.

