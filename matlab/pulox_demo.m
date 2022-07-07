function out = pulox_demo()
    data = [];
    start = datetime;
    function handler(x)
        data(end+1,:) = [uint64(milliseconds(datetime-start)) uint64(x)'];
    end
    p = Pulox(@handler);
    p.startRealtime();
    for i = 1:10
        pause(1);
        disp(length(data));
    end
    p.stopRealtime();
    plot( ...
        data(:, 1)/1000, data(:, 5), ...
        data(:, 1)/1000, data(:, 3), ...
        data(:, 1)/1000, data(:, 4));
    out = data;
end 

