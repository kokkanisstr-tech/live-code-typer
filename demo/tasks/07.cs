using System;
using System.Collections.Generic;

abstract class CollegeEmployee
{
    public string FullName { get; set; }
    public int EmployeeId { get; set; }
    public decimal Salary { get; set; }
    public int Experience { get; set; }

    public abstract void PerformDuty();
}

class Teacher : CollegeEmployee
{
    public string Subject { get; set; }

    public override void PerformDuty()
    {
        Console.WriteLine(FullName + " " + Subject + " пәнінен сабақ береді");
    }
}

class Librarian : CollegeEmployee
{
    public int BooksManaged { get; set; }

    public override void PerformDuty()
    {
        Console.WriteLine(FullName + " " + BooksManaged + " кітапқа жауапты");
    }
}

class Administrator : CollegeEmployee
{
    public string Department { get; set; }

    public override void PerformDuty()
    {
        Console.WriteLine(FullName + " " + Department + " бөлімінде әкімшілік жұмыс істейді");
    }
}

class Program
{
    static void Main()
    {
        List<CollegeEmployee> employees = new List<CollegeEmployee>();
        employees.Add(new Teacher { FullName = "Талдаубек Арай", EmployeeId = 1, Salary = 280000, Experience = 8, Subject = "Информатика" });
        employees.Add(new Teacher { FullName = "Асатова Асель", EmployeeId = 2, Salary = 260000, Experience = 12, Subject = "Программирование" });
        employees.Add(new Teacher { FullName = "Таңжарбаева Меруерт", EmployeeId = 3, Salary = 240000, Experience = 6, Subject = "Алгоритмы" });
        employees.Add(new Librarian { FullName = "Смагулова Гульнар", EmployeeId = 4, Salary = 150000, Experience = 15, BooksManaged = 5000 });
        employees.Add(new Librarian { FullName = "Ахметова Динара", EmployeeId = 5, Salary = 140000, Experience = 3, BooksManaged = 3200 });
        employees.Add(new Administrator { FullName = "Бекова Айгуль", EmployeeId = 6, Salary = 190000, Experience = 9, Department = "оку" });

        decimal fund = 0;
        int experienced = 0;
        foreach (CollegeEmployee employee in employees)
        {
            employee.PerformDuty();
            Console.WriteLine(employee.FullName + " - " + employee.Salary + " тг");
            fund = fund + employee.Salary;
            if (employee.Salary > 250000)
            {
                Console.WriteLine("  жалақысы 250 000-нан жоғары");
            }
            if (employee.Experience > 5)
            {
                experienced = experienced + 1;
            }
        }
        Console.WriteLine("5 жылдан астам өтілі барлар: " + experienced);
        Console.WriteLine("Жалақы қоры: " + fund + " тг");
    }
}
