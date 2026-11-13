using System;
using System.Collections.Generic;

abstract class HotelRoom
{
    public int RoomNumber { get; set; }
    public decimal PricePerNight { get; set; }
    public int Capacity { get; set; }
    public bool IsAvailable { get; set; }

    public abstract void ShowRoomInfo();
}

class SingleRoom : HotelRoom
{
    public bool HasDesk { get; set; }

    public override void ShowRoomInfo()
    {
        Console.WriteLine("Бір орындық бөлме №" + RoomNumber + ", " + PricePerNight + " тг/түн, жұмыс үстелі: " + (HasDesk ? "бар" : "жоқ"));
    }
}

class DoubleRoom : HotelRoom
{
    public bool HasBalcony { get; set; }

    public override void ShowRoomInfo()
    {
        Console.WriteLine("Екі орындық бөлме №" + RoomNumber + ", " + PricePerNight + " тг/түн, балкон: " + (HasBalcony ? "бар" : "жоқ"));
    }
}

class FamilyRoom : HotelRoom
{
    public bool HasKitchen { get; set; }

    public override void ShowRoomInfo()
    {
        Console.WriteLine("Отбасылық бөлме №" + RoomNumber + ", " + PricePerNight + " тг/түн, асүй: " + (HasKitchen ? "бар" : "жоқ"));
    }
}

class Program
{
    static void Main()
    {
        List<HotelRoom> rooms = new List<HotelRoom>();
        rooms.Add(new SingleRoom { RoomNumber = 101, PricePerNight = 12000, Capacity = 1, IsAvailable = true, HasDesk = true });
        rooms.Add(new SingleRoom { RoomNumber = 102, PricePerNight = 11000, Capacity = 1, IsAvailable = false, HasDesk = false });
        rooms.Add(new DoubleRoom { RoomNumber = 201, PricePerNight = 22000, Capacity = 2, IsAvailable = true, HasBalcony = true });
        rooms.Add(new DoubleRoom { RoomNumber = 202, PricePerNight = 25000, Capacity = 2, IsAvailable = true, HasBalcony = false });
        rooms.Add(new FamilyRoom { RoomNumber = 301, PricePerNight = 38000, Capacity = 4, IsAvailable = false, HasKitchen = true });
        rooms.Add(new FamilyRoom { RoomNumber = 302, PricePerNight = 42000, Capacity = 5, IsAvailable = true, HasKitchen = true });

        int available = 0;
        foreach (HotelRoom room in rooms)
        {
            room.ShowRoomInfo();
            if (room.IsAvailable)
            {
                available = available + 1;
                Console.WriteLine("  бос");
            }
            if (room.PricePerNight < 30000)
            {
                Console.WriteLine("  түндік бағасы 30 000 теңгеден төмен");
            }
            Console.WriteLine("  үш түн: " + (room.PricePerNight * 3) + " тг");
        }
        Console.WriteLine("Бос бөлмелер саны: " + available);
    }
}
