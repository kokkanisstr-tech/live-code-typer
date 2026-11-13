using System;
using System.Collections.Generic;

abstract class MusicalInstrument
{
    public string InstrumentName { get; set; }
    public string Manufacturer { get; set; }
    public decimal Price { get; set; }
    public int Year { get; set; }

    public abstract void Play();
}

class Piano : MusicalInstrument
{
    public int KeyCount { get; set; }

    public override void Play()
    {
        Console.WriteLine(InstrumentName + " ойнайды, пернелер саны " + KeyCount);
    }
}

class Guitar : MusicalInstrument
{
    public int StringCount { get; set; }

    public override void Play()
    {
        Console.WriteLine(InstrumentName + " ойнайды, ішектер саны " + StringCount);
    }
}

class Drum : MusicalInstrument
{
    public string DrumType { get; set; }

    public override void Play()
    {
        Console.WriteLine(InstrumentName + " ойнайды, түрі " + DrumType);
    }
}

class Program
{
    static void Main()
    {
        List<MusicalInstrument> instruments = new List<MusicalInstrument>();
        instruments.Add(new Piano { InstrumentName = "Рояль", Manufacturer = "Yamaha", Price = 850000, Year = 2019, KeyCount = 88 });
        instruments.Add(new Piano { InstrumentName = "Пианино", Manufacturer = "Kawai", Price = 450000, Year = 2022, KeyCount = 88 });
        instruments.Add(new Guitar { InstrumentName = "Акустикалық гитара", Manufacturer = "Fender", Price = 180000, Year = 2021, StringCount = 6 });
        instruments.Add(new Guitar { InstrumentName = "Классикалық гитара", Manufacturer = "Yamaha", Price = 90000, Year = 2018, StringCount = 6 });
        instruments.Add(new Drum { InstrumentName = "Барабан жинағы", Manufacturer = "Pearl", Price = 320000, Year = 2023, DrumType = "акустикалық" });
        instruments.Add(new Drum { InstrumentName = "Электронды барабан", Manufacturer = "Roland", Price = 150000, Year = 2024, DrumType = "электронды" });

        decimal total = 0;
        foreach (MusicalInstrument instrument in instruments)
        {
            instrument.Play();
            Console.WriteLine("  " + instrument.Manufacturer + ", " + instrument.Price + " тг, " + instrument.Year + " жыл");
            total = total + instrument.Price;
            if (instrument.Price > 100000)
            {
                Console.WriteLine("  бағасы 100 000 теңгеден жоғары");
            }
            if (instrument.Year > 2020)
            {
                Console.WriteLine("  2020 жылдан кейін шыққан");
            }
        }
        Console.WriteLine("Жалпы бағасы: " + total + " тг");
    }
}
