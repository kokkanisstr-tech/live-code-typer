using System;
using System.Collections.Generic;

abstract class Machine
{
    public string MachineName { get; set; }
    public double Power { get; set; }
    public bool IsWorking { get; set; }
    public int OperatingHours { get; set; }

    public abstract void StartWork();
}

class CuttingMachine : Machine
{
    public double CuttingSpeed { get; set; }

    public override void StartWork()
    {
        Console.WriteLine("Кесу машинасы " + MachineName + " іске қосылды, жылдамдық " + CuttingSpeed);
    }
}

class DrillingMachine : Machine
{
    public double DrillDiameter { get; set; }

    public override void StartWork()
    {
        Console.WriteLine("Бұрғылау машинасы " + MachineName + " іске қосылды, диаметр " + DrillDiameter);
    }
}

class PrintingMachine : Machine
{
    public int PrintSpeed { get; set; }

    public override void StartWork()
    {
        Console.WriteLine("Басып шығару машинасы " + MachineName + " іске қосылды, жылдамдық " + PrintSpeed);
    }
}

class Program
{
    static void Main()
    {
        List<Machine> machines = new List<Machine>();
        machines.Add(new CuttingMachine { MachineName = "Cutter-1", Power = 7.5, IsWorking = true, OperatingHours = 1200, CuttingSpeed = 3.5 });
        machines.Add(new CuttingMachine { MachineName = "Cutter-2", Power = 5.0, IsWorking = false, OperatingHours = 800, CuttingSpeed = 2.0 });
        machines.Add(new DrillingMachine { MachineName = "Drill-1", Power = 3.0, IsWorking = true, OperatingHours = 1500, DrillDiameter = 12 });
        machines.Add(new DrillingMachine { MachineName = "Drill-2", Power = 2.5, IsWorking = true, OperatingHours = 400, DrillDiameter = 8 });
        machines.Add(new PrintingMachine { MachineName = "Printer-1", Power = 1.5, IsWorking = false, OperatingHours = 2000, PrintSpeed = 120 });
        machines.Add(new PrintingMachine { MachineName = "Printer-2", Power = 2.0, IsWorking = true, OperatingHours = 900, PrintSpeed = 150 });

        double totalPower = 0;
        foreach (Machine machine in machines)
        {
            Console.WriteLine(machine.MachineName + ", қуат " + machine.Power + " кВт, " + machine.OperatingHours + " сағат");
            totalPower = totalPower + machine.Power;
            if (machine.IsWorking == false)
            {
                Console.WriteLine("  жұмыс істемейді");
            }
            else
            {
                machine.StartWork();
            }
            if (machine.OperatingHours > 1000)
            {
                Console.WriteLine("  1000 сағаттан көп жұмыс істеген");
            }
        }
        Console.WriteLine("Жалпы қуат: " + totalPower + " кВт");
    }
}
