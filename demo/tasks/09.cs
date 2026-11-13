using System;
using System.Collections.Generic;

abstract class PublicTransport
{
    public string RouteNumber { get; set; }
    public int PassengerCapacity { get; set; }
    public int CurrentPassengers { get; set; }
    public decimal TicketPrice { get; set; }

    public abstract void ShowRoute();
}

class Bus : PublicTransport
{
    public string FuelType { get; set; }

    public override void ShowRoute()
    {
        Console.WriteLine("Автобус №" + RouteNumber + ", отын: " + FuelType + ", жолаушы " + CurrentPassengers + "/" + PassengerCapacity);
    }
}

class Tram : PublicTransport
{
    public int TrackNumber { get; set; }

    public override void ShowRoute()
    {
        Console.WriteLine("Трамвай №" + RouteNumber + ", жолы " + TrackNumber + ", жолаушы " + CurrentPassengers + "/" + PassengerCapacity);
    }
}

class Metro : PublicTransport
{
    public string LineName { get; set; }

    public override void ShowRoute()
    {
        Console.WriteLine("Метро №" + RouteNumber + ", " + LineName + " желісі, жолаушы " + CurrentPassengers + "/" + PassengerCapacity);
    }
}

class Program
{
    static void Main()
    {
        List<PublicTransport> transports = new List<PublicTransport>();
        transports.Add(new Bus { RouteNumber = "12", PassengerCapacity = 50, CurrentPassengers = 45, TicketPrice = 150, FuelType = "дизель" });
        transports.Add(new Bus { RouteNumber = "35", PassengerCapacity = 50, CurrentPassengers = 55, TicketPrice = 180, FuelType = "газ" });
        transports.Add(new Tram { RouteNumber = "4", PassengerCapacity = 120, CurrentPassengers = 100, TicketPrice = 120, TrackNumber = 2 });
        transports.Add(new Tram { RouteNumber = "7", PassengerCapacity = 120, CurrentPassengers = 130, TicketPrice = 120, TrackNumber = 3 });
        transports.Add(new Metro { RouteNumber = "1", PassengerCapacity = 300, CurrentPassengers = 250, TicketPrice = 100, LineName = "бірінші" });
        transports.Add(new Metro { RouteNumber = "2", PassengerCapacity = 300, CurrentPassengers = 180, TicketPrice = 100, LineName = "екінші" });

        foreach (PublicTransport transport in transports)
        {
            transport.ShowRoute();
            if (transport.CurrentPassengers > transport.PassengerCapacity)
            {
                Console.WriteLine("  сыйымдылықтан асып кетті");
            }
            if (transport.TicketPrice < 200)
            {
                Console.WriteLine("  билет 200 теңгеден төмен");
            }
            decimal income = transport.CurrentPassengers * transport.TicketPrice;
            Console.WriteLine("  ықтимал табыс: " + income + " тг");
        }
    }
}
