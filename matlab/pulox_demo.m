function out = pulox_demo()
    data = [];
    start = datetime;
    % Callback which gets called everytime a new measurement arrives
    function handler(x, ~)
        % Save the current time and the measurement value in data
        data(end+1,:) = [uint64(milliseconds(datetime-start)) uint64(x)'];
    end
    % Connect to device and request real time data
    p = Pulox(@handler);
    p.startRealtime();
    % Wait for approximately 10 seconds, periodically printing the number of received measurements
    for i = 1:10
        pause(1);
        disp(length(data));
    end
    % Stop real time data
    p.stopRealtime();
    % Plot the received data
    plot( ...
        data(:, 1)/1000, data(:, 5), ...
        data(:, 1)/1000, data(:, 3), ...
        data(:, 1)/1000, data(:, 4));
    out = data;
end 

