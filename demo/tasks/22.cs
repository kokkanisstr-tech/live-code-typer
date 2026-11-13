using System;
using System.Collections.Generic;

abstract class Equipment
{
    public string EquipmentName { get; set; }
    public int InventoryNumber { get; set; }
    public decimal Price { get; set; }
    public bool NeedsRepair { get; set; }

    public abstract void CheckStatus();
}

class Computer : Equipment
{
    public int Ram { get; set; }

    public override void CheckStatus()
    {
        Console.WriteLine("Компьютер " + EquipmentName + " (инв. " + InventoryNumber + "), RAM " + Ram + " ГБ");
    }
}

class Projector : Equipment
{
    public int Brightness { get; set; }

    public override void CheckStatus()
    {
        Console.WriteLine("Проектор " + EquipmentName + " (инв. " + InventoryNumber + "), жарықтық " + Brightness + " лм");
    }
}

class Printer : Equipment
{
    public bool IsColor { get; set; }

    public override void CheckStatus()
    {
        Console.WriteLine("Принтер " + EquipmentName + " (инв. " + InventoryNumber + "), түрлі түсті: " + (IsColor ? "иә" : "жоқ"));
    }
}

class Program
{
    static void Main()
    {
        List<Equipment> items = new List<Equipment>();
        items.Add(new Computer { EquipmentName = "ПК-1", InventoryNumber = 1001, Price = 320000, NeedsRepair = false, Ram = 16 });
        items.Add(new Computer { EquipmentName = "ПК-2", InventoryNumber = 1002, Price = 280000, NeedsRepair = true, Ram = 8 });
        items.Add(new Projector { EquipmentName = "Epson", InventoryNumber = 2001, Price = 180000, NeedsRepair = false, Brightness = 3500 });
        items.Add(new Projector { EquipmentName = "BenQ", InventoryNumber = 2002, Price = 140000, NeedsRepair = true, Brightness = 3000 });
        items.Add(new Printer { EquipmentName = "HP LaserJet", InventoryNumber = 3001, Price = 95000, NeedsRepair = false, IsColor = false });
        items.Add(new Printer { EquipmentName = "Canon", InventoryNumber = 3002, Price = 160000, NeedsRepair = false, IsColor = true });

        decimal total = 0;
        int repair = 0;
        foreach (Equipment item in items)
        {
            item.CheckStatus();
            total = total + item.Price;
            if (item.NeedsRepair)
            {
                repair = repair + 1;
                Console.WriteLine("  жөндеу қажет");
            }
            if (item.Price > 150000)
            {
                Console.WriteLine("  бағасы 150 000 теңгеден жоғары");
            }
        }
        Console.WriteLine("Жөндеу қажет жабдықтар: " + repair);
        Console.WriteLine("Жалпы бағасы: " + total + " тг");
    }
}
