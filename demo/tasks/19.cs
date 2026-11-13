using System;
using System.Collections.Generic;

abstract class Tour
{
    public string TourName { get; set; }
    public string Destination { get; set; }
    public int Duration { get; set; }
    public decimal PricePerPerson { get; set; }
    public int ParticipantCount { get; set; }

    public abstract void ShowItinerary();
}

class MountainTour : Tour
{
    public string DifficultyLevel { get; set; }

    public override void ShowItinerary()
    {
        Console.WriteLine("Тау туры: " + TourName + ", " + Destination + ", " + Duration + " күн, қиындық " + DifficultyLevel);
    }
}

class CityTour : Tour
{
    public string CityName { get; set; }

    public override void ShowItinerary()
    {
        Console.WriteLine("Қала туры: " + TourName + ", " + CityName + ", " + Duration + " күн, " + PricePerPerson + " тг");
    }
}

class SeaTour : Tour
{
    public string SeaName { get; set; }

    public override void ShowItinerary()
    {
        Console.WriteLine("Теңіз туры: " + TourName + ", " + SeaName + ", " + Duration + " күн, " + PricePerPerson + " тг");
    }
}

class Program
{
    static void Main()
    {
        List<Tour> tours = new List<Tour>();
        tours.Add(new MountainTour { TourName = "Алатау трек", Destination = "Алматы", Duration = 3, PricePerPerson = 85000, ParticipantCount = 12, DifficultyLevel = "орташа" });
        tours.Add(new MountainTour { TourName = "Хан Тәңірі", Destination = "Каракол", Duration = 7, PricePerPerson = 250000, ParticipantCount = 6, DifficultyLevel = "қиын" });
        tours.Add(new CityTour { TourName = "Астана сити", Destination = "Астана", Duration = 2, PricePerPerson = 60000, ParticipantCount = 20, CityName = "Астана" });
        tours.Add(new CityTour { TourName = "Алматы тур", Destination = "Алматы", Duration = 1, PricePerPerson = 35000, ParticipantCount = 30, CityName = "Алматы" });
        tours.Add(new SeaTour { TourName = "Каспий демалысы", Destination = "Актау", Duration = 5, PricePerPerson = 120000, ParticipantCount = 15, SeaName = "Каспий" });
        tours.Add(new SeaTour { TourName = "Анталья", Destination = "Турция", Duration = 8, PricePerPerson = 320000, ParticipantCount = 10, SeaName = "Жерорта" });

        decimal totalIncome = 0;
        foreach (Tour tour in tours)
        {
            tour.ShowItinerary();
            decimal income = tour.PricePerPerson * tour.ParticipantCount;
            totalIncome = totalIncome + income;
            Console.WriteLine("  тур табысы: " + income + " тг");
            if (tour.Duration > 5)
            {
                Console.WriteLine("  ұзақтығы 5 күннен асады");
            }
            if (tour.PricePerPerson < 100000)
            {
                Console.WriteLine("  бағасы 100 000 теңгеден төмен");
            }
        }
        Console.WriteLine("Барлық турлардан түсетін табыс: " + totalIncome + " тг");
    }
}
