using System;
using System.Collections.Generic;

abstract class Flight
{
    public string FlightNumber { get; set; }
    public string DepartureCity { get; set; }
    public string ArrivalCity { get; set; }
    public decimal TicketPrice { get; set; }
    public bool IsDelayed { get; set; }

    public abstract void ShowFlightInfo();
}

class DomesticFlight : Flight
{
    public string Region { get; set; }

    public override void ShowFlightInfo()
    {
        Console.WriteLine("Ішкі рейс " + FlightNumber + ": " + DepartureCity + " - " + ArrivalCity + ", " + Region + " аймағы, " + TicketPrice + " тг");
    }
}

class InternationalFlight : Flight
{
    public string Country { get; set; }

    public override void ShowFlightInfo()
    {
        Console.WriteLine("Халықаралық рейс " + FlightNumber + ": " + DepartureCity + " - " + ArrivalCity + ", " + Country + ", " + TicketPrice + " тг");
    }
}

class CharterFlight : Flight
{
    public string TourOperator { get; set; }

    public override void ShowFlightInfo()
    {
        Console.WriteLine("Чартерлік рейс " + FlightNumber + ": " + DepartureCity + " - " + ArrivalCity + ", " + TourOperator + ", " + TicketPrice + " тг");
    }
}

class Program
{
    static void Main()
    {
        List<Flight> flights = new List<Flight>();
        flights.Add(new DomesticFlight { FlightNumber = "KC101", DepartureCity = "Алматы", ArrivalCity = "Астана", TicketPrice = 45000, IsDelayed = false, Region = "оңтүстік" });
        flights.Add(new DomesticFlight { FlightNumber = "KC102", DepartureCity = "Астана", ArrivalCity = "Шымкент", TicketPrice = 38000, IsDelayed = true, Region = "оңтүстік" });
        flights.Add(new InternationalFlight { FlightNumber = "KC201", DepartureCity = "Алматы", ArrivalCity = "Стамбул", TicketPrice = 180000, IsDelayed = false, Country = "Түркия" });
        flights.Add(new InternationalFlight { FlightNumber = "KC202", DepartureCity = "Астана", ArrivalCity = "Дубай", TicketPrice = 220000, IsDelayed = true, Country = "БАӘ" });
        flights.Add(new CharterFlight { FlightNumber = "KC301", DepartureCity = "Алматы", ArrivalCity = "Анталья", TicketPrice = 150000, IsDelayed = false, TourOperator = "SunTour" });
        flights.Add(new CharterFlight { FlightNumber = "KC302", DepartureCity = "Астана", ArrivalCity = "Шарм", TicketPrice = 190000, IsDelayed = false, TourOperator = "TravelKZ" });

        int delayed = 0;
        decimal sum = 0;
        foreach (Flight flight in flights)
        {
            flight.ShowFlightInfo();
            sum = sum + flight.TicketPrice;
            if (flight.IsDelayed)
            {
                delayed = delayed + 1;
                Console.WriteLine("  кешігуде");
            }
            if (flight.TicketPrice > 100000)
            {
                Console.WriteLine("  билет 100 000 теңгеден жоғары");
            }
        }
        Console.WriteLine("Кешіккен рейстер: " + delayed);
        Console.WriteLine("Орташа билет бағасы: " + (sum / flights.Count) + " тг");
    }
}
