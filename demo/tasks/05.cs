using System;
using System.Collections.Generic;

abstract class MedicalService
{
    public string ServiceName { get; set; }
    public decimal Price { get; set; }
    public int Duration { get; set; }
    public string DoctorName { get; set; }

    public abstract decimal CalculateCost();
}

class Consultation : MedicalService
{
    public string ConsultationType { get; set; }

    public override decimal CalculateCost()
    {
        return Price + Price * 10 / 100;
    }
}

class Analysis : MedicalService
{
    public int AnalysisCount { get; set; }

    public override decimal CalculateCost()
    {
        return Price * AnalysisCount;
    }
}

class Ultrasound : MedicalService
{
    public string BodyPart { get; set; }

    public override decimal CalculateCost()
    {
        return Price + Price * 20 / 100;
    }
}

class Program
{
    static void Main()
    {
        List<MedicalService> services = new List<MedicalService>();
        services.Add(new Consultation { ServiceName = "Терапевт", Price = 5000, Duration = 20, DoctorName = "Ахметова А.", ConsultationType = "алгашкы" });
        services.Add(new Consultation { ServiceName = "Кардиолог", Price = 8000, Duration = 25, DoctorName = "Сериков Б.", ConsultationType = "кайталама" });
        services.Add(new Analysis { ServiceName = "Қан талдауы", Price = 3000, Duration = 10, DoctorName = "Оспанова Г.", AnalysisCount = 2 });
        services.Add(new Analysis { ServiceName = "Зәр талдауы", Price = 2500, Duration = 10, DoctorName = "Оспанова Г.", AnalysisCount = 3 });
        services.Add(new Ultrasound { ServiceName = "Іш қуысы", Price = 12000, Duration = 30, DoctorName = "Нурланова Д.", BodyPart = "іш" });
        services.Add(new Ultrasound { ServiceName = "Жүрек УДЗ", Price = 15000, Duration = 35, DoctorName = "Нурланова Д.", BodyPart = "жүрек" });

        decimal total = 0;
        foreach (MedicalService service in services)
        {
            decimal cost = service.CalculateCost();
            total = total + cost;
            Console.WriteLine(service.ServiceName + " - " + cost + " тг");
            if (cost > 15000)
            {
                Console.WriteLine("  қымбат қызмет");
            }
        }
        Console.WriteLine("Жалпы құны: " + total + " тг");
    }
}
