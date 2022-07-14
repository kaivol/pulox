classdef Pulox < handle
    %PULOX Represents a connection to a Pulox PPG device

    properties (Access = private)
        port
        state uint8
        callback function_handle
        wake
    end
    
    methods
        function obj = Pulox(callback, port)
            arguments
                callback function_handle
                port (1,:) char = 'COM3'
            end
            %PULOX Construct an instance of this class
            %   Opens the connection with a hardware device.
            %   You must ensure that a compatible PPG is connected at the given serial port.
            %
            %   1. Argument (mandatory): callback
            %       Callback which gets executed everytime a new measurement is received from the device
            %       The callback is invoked with the following arguments:
            %       1. measurement data: uint8 array [probe_errors, spo2, pulse_rate, pulse_waveform]
            %       2. The Pulox object
            %   2. Argument (optional): port
            %       Name of serial port, defaults to 'COM3'

            % Initialize state machine
            obj.state = mex_pulox(uint64(3), uint8.empty);
            obj.callback = callback;
            obj.port = serialport(port, 115200);
        end
        
        function startRealtime(obj)
            %startRealtime Start real time data
            %   Instructs the device to start sending real time data.
            %   You need to make sure that the device is turned on and ready to start sending
            %   real time data.
            function callback(~, ~)
                % Send InformDeviceConnected every 4 seconds
                if seconds(datetime("now") - obj.wake) > 4
                    obj.wake = datetime("now");
                    % Get InformDeviceConnected package from library and send it
                    bytes = mex_pulox(uint64(2), uint8.empty);
                    write(obj.port, bytes, "uint8");
                    disp("Send Keepalive");
                end
                
                % Resume state machine with new data
                res = mex_pulox(uint64(4), obj);
                if length(res) == 1
                    % No complete package yet
                    if res(1) == 0
                    end
                    % Got FreeFeedback, disconnect callback
                    if res(1) == 1
                        configureCallback(obj.port, "off");
                    end
                else
                    % Got sample
                    obj.callback(res, obj);
                end
            end
            % Discard all leftover bytes
            if obj.port.NumBytesAvailable > 0
                read(obj.port, obj.port.NumBytesAvailable, "uint8");
                end
            % COnfigure callback of serial port
            configureCallback(obj.port, "byte", 1, @callback);

            obj.wake = datetime("now");

            % Get ContinuousRealTimeData package from library and send it
            bytes = mex_pulox(uint64(0), uint8.empty);
            write(obj.port, bytes, "uint8");
        end
        
        function stopRealtime(obj)
            %stopRealtime Stop real time data
            %   Instructs the device to stop sending real time data.
            %   Note that there could be still incoming measurements after the call to this method.

            % Get StopRealTimeData package from library and send it
            bytes = mex_pulox(uint64(1), uint8.empty);
            write(obj.port, bytes, "uint8");
        end
    end
end

