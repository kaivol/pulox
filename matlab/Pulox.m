classdef Pulox < handle
    %PULOX Summary of this class goes here
    %   Detailed explanation goes here
    
    properties (SetAccess = private)
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
            %   Opens the connection with a hardware device
            obj.state = mex_pulox(uint64(3), uint8.empty);
            obj.callback = callback;
            obj.port = serialport(port, 115200);
        end
        
        function startRealtime(obj)
            %startRealtime Start real time data
            %   You need to make sure that the device is turned on
            function callback(~, ~)
                % Send InformDeviceConnected every 4 seconds
                if seconds(datetime("now") - obj.wake) > 4
                    obj.wake = datetime("now");
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
                    obj.callback(res);
                end
                   
            end
            if obj.port.NumBytesAvailable > 0

                read(obj.port, obj.port.NumBytesAvailable, "uint8");
            end
            configureCallback(obj.port, "byte", 1, @callback);

            obj.wake = datetime("now");

            % Send ContinuousRealTimeData
            bytes = mex_pulox(uint64(0), uint8.empty);
            write(obj.port, bytes, "uint8");
        end
        
        function stopRealtime(obj)
            %METHOD1 Stop real time data

            % Send StopRealTimeData
            bytes = mex_pulox(uint64(1), uint8.empty);
            write(obj.port, bytes, "uint8");
        end
    end
end

