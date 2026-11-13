using System;
using System.Collections.Generic;

abstract class Robot
{
    public string RobotName { get; set; }
    public int BatteryLevel { get; set; }
    public double Speed { get; set; }
    public bool IsActive { get; set; }

    public abstract void ExecuteTask();
}

class CleaningRobot : Robot
{
    public double CleaningArea { get; set; }

    public override void ExecuteTask()
    {
        Console.WriteLine(RobotName + " " + CleaningArea + " шаршы метр тазалайды");
    }
}

class DeliveryRobot : Robot
{
    public int PackageCount { get; set; }

    public override void ExecuteTask()
    {
        Console.WriteLine(RobotName + " " + PackageCount + " сәлемдеме жеткізеді");
    }
}

class SecurityRobot : Robot
{
    public string PatrolArea { get; set; }

    public override void ExecuteTask()
    {
        Console.WriteLine(RobotName + " " + PatrolArea + " аумағын күзетеді");
    }
}

class Program
{
    static void Main()
    {
        List<Robot> robots = new List<Robot>();
        robots.Add(new CleaningRobot { RobotName = "Cleaner-1", BatteryLevel = 80, Speed = 0.5, IsActive = true, CleaningArea = 120 });
        robots.Add(new CleaningRobot { RobotName = "Cleaner-2", BatteryLevel = 15, Speed = 0.4, IsActive = false, CleaningArea = 90 });
        robots.Add(new DeliveryRobot { RobotName = "Courier-1", BatteryLevel = 60, Speed = 2.5, IsActive = true, PackageCount = 12 });
        robots.Add(new DeliveryRobot { RobotName = "Courier-2", BatteryLevel = 10, Speed = 2.0, IsActive = false, PackageCount = 5 });
        robots.Add(new SecurityRobot { RobotName = "Guard-1", BatteryLevel = 95, Speed = 1.2, IsActive = true, PatrolArea = "колледж" });
        robots.Add(new SecurityRobot { RobotName = "Guard-2", BatteryLevel = 45, Speed = 1.0, IsActive = true, PatrolArea = "қойма" });

        int active = 0;
        foreach (Robot robot in robots)
        {
            robot.ExecuteTask();
            if (robot.IsActive)
            {
                active = active + 1;
            }
            if (robot.BatteryLevel < 20)
            {
                Console.WriteLine("  батарея 20%-дан төмен");
            }
            if (robot.BatteryLevel > 50)
            {
                Console.WriteLine("  батарея 50%-дан жоғары");
            }
        }
        Console.WriteLine("Белсенді роботтар: " + active);
    }
}
