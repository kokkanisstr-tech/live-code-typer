using System;
using System.Collections.Generic;

abstract class Device
{
    public string Brand { get; set; }
    public string Model { get; set; }
    public decimal Price { get; set; }
    public bool IsWorking { get; set; }

    public abstract void TurnOn();
    public abstract void ShowSpecifications();
}

class Laptop : Device
{
    public int Ram { get; set; }

    public override void TurnOn()
    {
        Console.WriteLine(Brand + " " + Model + " қосылды");
    }

    public override void ShowSpecifications()
    {
        Console.WriteLine(Brand + " " + Model + ", RAM " + Ram + " ГБ, " + Price + " тг");
    }
}

class Smartphone : Device
{
    public int BatteryCapacity { get; set; }

    public override void TurnOn()
    {
        Console.WriteLine(Brand + " " + Model + " қосылды");
    }

    public override void ShowSpecifications()
    {
        Console.WriteLine(Brand + " " + Model + ", батарея " + BatteryCapacity + " мАч, " + Price + " тг");
    }
}

class Tablet : Device
{
    public double ScreenSize { get; set; }

    public override void TurnOn()
    {
        Console.WriteLine(Brand + " " + Model + " қосылды");
    }

    public override void ShowSpecifications()
    {
        Console.WriteLine(Brand + " " + Model + ", экран " + ScreenSize + " дюйм, " + Price + " тг");
    }
}

class Program
{
    static void Main()
    {
        List<Device> devices = new List<Device>();
        devices.Add(new Laptop { Brand = "Lenovo", Model = "IdeaPad", Price = 320000, IsWorking = true, Ram = 16 });
        devices.Add(new Laptop { Brand = "Asus", Model = "VivoBook", Price = 280000, IsWorking = false, Ram = 8 });
        devices.Add(new Smartphone { Brand = "Samsung", Model = "A54", Price = 180000, IsWorking = true, BatteryCapacity = 5000 });
        devices.Add(new Smartphone { Brand = "Xiaomi", Model = "Redmi 12", Price = 90000, IsWorking = true, BatteryCapacity = 5000 });
        devices.Add(new Tablet { Brand = "Apple", Model = "iPad Air", Price = 450000, IsWorking = true, ScreenSize = 10.9 });
        devices.Add(new Tablet { Brand = "Huawei", Model = "MatePad", Price = 150000, IsWorking = false, ScreenSize = 10.4 });

        decimal total = 0;
        int broken = 0;
        foreach (Device device in devices)
        {
            device.ShowSpecifications();
            device.TurnOn();
            total = total + device.Price;
            if (device.IsWorking == false)
            {
                broken = broken + 1;
            }
        }

        Console.WriteLine("--- Жұмыс істемейтін құрылғылар: " + broken);
        Console.WriteLine("--- Бағасы 200 000 теңгеден жоғары ---");
        foreach (Device device in devices)
        {
            if (device.Price > 200000)
            {
                Console.WriteLine(device.Brand + " " + device.Model + " - " + device.Price);
            }
        }
        Console.WriteLine("Жалпы бағасы: " + total + " тг");
    }
}
