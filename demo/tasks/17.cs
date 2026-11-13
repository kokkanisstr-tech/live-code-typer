using System;
using System.Collections.Generic;

abstract class NetworkDevice
{
    public string DeviceName { get; set; }
    public string IpAddress { get; set; }
    public bool IsConnected { get; set; }
    public int Speed { get; set; }

    public abstract void Connect();
}

class Router : NetworkDevice
{
    public int PortCount { get; set; }

    public override void Connect()
    {
        Console.WriteLine("Маршрутизатор " + DeviceName + " қосылды, порттар: " + PortCount);
    }
}

class Switch : NetworkDevice
{
    public bool Managed { get; set; }

    public override void Connect()
    {
        Console.WriteLine("Коммутатор " + DeviceName + " қосылды, басқарылады: " + (Managed ? "иә" : "жоқ"));
    }
}

class AccessPoint : NetworkDevice
{
    public string WifiStandard { get; set; }

    public override void Connect()
    {
        Console.WriteLine("Қолжетімділік нүктесі " + DeviceName + " қосылды, стандарт " + WifiStandard);
    }
}

class Program
{
    static void Main()
    {
        List<NetworkDevice> devices = new List<NetworkDevice>();
        devices.Add(new Router { DeviceName = "Router-1", IpAddress = "192.168.1.1", IsConnected = true, Speed = 1000, PortCount = 4 });
        devices.Add(new Router { DeviceName = "Router-2", IpAddress = "192.168.2.1", IsConnected = false, Speed = 300, PortCount = 8 });
        devices.Add(new Switch { DeviceName = "Switch-1", IpAddress = "192.168.1.2", IsConnected = true, Speed = 1000, Managed = true });
        devices.Add(new Switch { DeviceName = "Switch-2", IpAddress = "192.168.1.3", IsConnected = true, Speed = 100, Managed = false });
        devices.Add(new AccessPoint { DeviceName = "AP-1", IpAddress = "192.168.1.10", IsConnected = true, Speed = 300, WifiStandard = "Wi-Fi 5" });
        devices.Add(new AccessPoint { DeviceName = "AP-2", IpAddress = "192.168.1.11", IsConnected = false, Speed = 1200, WifiStandard = "Wi-Fi 6" });

        int connected = 0;
        foreach (NetworkDevice device in devices)
        {
            Console.WriteLine(device.DeviceName + " - " + device.IpAddress + ", " + device.Speed + " Мбит/с");
            if (device.IsConnected)
            {
                device.Connect();
                connected = connected + 1;
            }
            else
            {
                Console.WriteLine("  қосылмаған");
            }
            if (device.Speed > 100)
            {
                Console.WriteLine("  жылдамдығы 100 Мбит/с-тан жоғары");
            }
        }
        Console.WriteLine("Қосылған құрылғылар: " + connected);
    }
}
