using System;
using System.Collections.Generic;

abstract class LibraryItem
{
    public string Title { get; set; }
    public string Author { get; set; }
    public int Year { get; set; }
    public bool IsAvailable { get; set; }

    public abstract void DisplayInfo();
}

class Book : LibraryItem
{
    public int PageCount { get; set; }

    public override void DisplayInfo()
    {
        Console.WriteLine("Кітап: " + Title + ", " + Author + ", " + Year + ", " + PageCount + " бет, " + (IsAvailable ? "колжетімді" : "берілген"));
    }
}

class Magazine : LibraryItem
{
    public int IssueNumber { get; set; }

    public override void DisplayInfo()
    {
        Console.WriteLine("Журнал: " + Title + ", " + Author + ", " + Year + ", №" + IssueNumber + ", " + (IsAvailable ? "колжетімді" : "берілген"));
    }
}

class Newspaper : LibraryItem
{
    public string PublicationDate { get; set; }

    public override void DisplayInfo()
    {
        Console.WriteLine("Газет: " + Title + ", " + Author + ", " + Year + ", " + PublicationDate + ", " + (IsAvailable ? "колжетімді" : "берілген"));
    }
}

class Program
{
    static void Main()
    {
        List<LibraryItem> items = new List<LibraryItem>();
        items.Add(new Book { Title = "Абай жолы", Author = "М. Ауезов", Year = 1942, IsAvailable = true, PageCount = 720 });
        items.Add(new Book { Title = "Кан мен тер", Author = "А. Нурпеисов", Year = 1970, IsAvailable = false, PageCount = 640 });
        items.Add(new Magazine { Title = "Наука и жизнь", Author = "Редакция", Year = 2024, IsAvailable = true, IssueNumber = 5 });
        items.Add(new Magazine { Title = "Вокруг света", Author = "Редакция", Year = 2023, IsAvailable = false, IssueNumber = 12 });
        items.Add(new Newspaper { Title = "Егемен Казакстан", Author = "Редакция", Year = 2026, IsAvailable = true, PublicationDate = "10.11.2026" });
        items.Add(new Newspaper { Title = "Казахстанская правда", Author = "Редакция", Year = 2026, IsAvailable = true, PublicationDate = "09.11.2026" });

        foreach (LibraryItem item in items)
        {
            item.DisplayInfo();
        }

        Console.WriteLine("--- Колжетімді материалдар ---");
        int available = 0;
        foreach (LibraryItem item in items)
        {
            if (item.IsAvailable)
            {
                Console.WriteLine(item.Title);
                available = available + 1;
            }
        }
        Console.WriteLine("Колжетімді саны: " + available);
    }
}
