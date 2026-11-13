using System;
using System.Collections.Generic;

abstract class Sensor
{
    public string SensorName { get; set; }
    public string Location { get; set; }
    public bool IsActive { get; set; }
    public double LastReading { get; set; }

    public abstract void GetReading();
}

class TemperatureSensor : Sensor
{
    public double Temperature { get; set; }

    public override void GetReading()
    {
        LastReading = Temperature;
        Console.WriteLine("Температура датчигі " + SensorName + " (" + Location + "): " + Temperature + " C");
    }
}

class HumiditySensor : Sensor
{
    public int Humidity { get; set; }

    public override void GetReading()
    {
        LastReading = Humidity;
        Console.WriteLine("Ылғалдылық датчигі " + SensorName + " (" + Location + "): " + Humidity + "%");
    }
}

class AirQualitySensor : Sensor
{
    public int PollutionLevel { get; set; }

    public override void GetReading()
    {
        LastReading = PollutionLevel;
        Console.WriteLine("Ауа сапасы датчигі " + SensorName + " (" + Location + "): ластану " + PollutionLevel);
    }
}

class Program
{
    static void Main()
    {
        List<Sensor> sensors = new List<Sensor>();
        sensors.Add(new TemperatureSensor { SensorName = "T-1", Location = "Аудитория 101", IsActive = true, Temperature = 22 });
        sensors.Add(new TemperatureSensor { SensorName = "T-2", Location = "Қазандық", IsActive = true, Temperature = 38 });
        sensors.Add(new HumiditySensor { SensorName = "H-1", Location = "Аудитория 101", IsActive = true, Humidity = 45 });
        sensors.Add(new HumiditySensor { SensorName = "H-2", Location = "Қойма", IsActive = false, Humidity = 60 });
        sensors.Add(new AirQualitySensor { SensorName = "A-1", Location = "Кіреберіс", IsActive = true, PollutionLevel = 12 });
        sensors.Add(new AirQualitySensor { SensorName = "A-2", Location = "Асхана", IsActive = false, PollutionLevel = 30 });

        int active = 0;
        foreach (Sensor sensor in sensors)
        {
            sensor.GetReading();
            if (sensor.IsActive)
            {
                active = active + 1;
            }
            else
            {
                Console.WriteLine("  белсенді емес");
            }
            TemperatureSensor temp = sensor as TemperatureSensor;
            if (temp != null && temp.Temperature > 35)
            {
                Console.WriteLine("  ЕСКЕРТУ: " + temp.Location + " жерінде температура 35 C-тан жоғары");
            }
        }
        Console.WriteLine("Белсенді датчиктер: " + active);
    }
}
