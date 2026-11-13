using System;
using System.Collections.Generic;

abstract class TestCase
{
    public string TestName { get; set; }
    public bool IsPassed { get; set; }

    public abstract void RunTest();
}

class LoginTest : TestCase
{
    public string Username { get; set; }
    public string Password { get; set; }

    public override void RunTest()
    {
        if (Username != "" && Password.Length >= 6)
        {
            IsPassed = true;
        }
        else
        {
            IsPassed = false;
        }
    }
}

class RegistrationTest : TestCase
{
    public string Username { get; set; }
    public string Email { get; set; }

    public override void RunTest()
    {
        if (Username != "" && Email != "")
        {
            IsPassed = true;
        }
        else
        {
            IsPassed = false;
        }
    }
}

class PaymentTest : TestCase
{
    public decimal PaymentAmount { get; set; }

    public override void RunTest()
    {
        if (PaymentAmount > 0)
        {
            IsPassed = true;
        }
        else
        {
            IsPassed = false;
        }
    }
}

class Program
{
    static void Main()
    {
        List<TestCase> tests = new List<TestCase>();
        tests.Add(new LoginTest { TestName = "Login дұрыс", Username = "yusuf", Password = "123456" });
        tests.Add(new LoginTest { TestName = "Login қысқа құпиясөз", Username = "yusuf", Password = "123" });
        tests.Add(new RegistrationTest { TestName = "Тіркелу дұрыс", Username = "yusuf", Email = "yusuf@mail.kz" });
        tests.Add(new RegistrationTest { TestName = "Тіркелу email жоқ", Username = "yusuf", Email = "" });
        tests.Add(new PaymentTest { TestName = "Төлем дұрыс", PaymentAmount = 5000 });
        tests.Add(new PaymentTest { TestName = "Төлем нөл", PaymentAmount = 0 });

        int passed = 0;
        int failed = 0;
        foreach (TestCase test in tests)
        {
            test.RunTest();
            string result = test.IsPassed ? "сәтті" : "сәтсіз";
            Console.WriteLine(test.TestName + " - " + result);
            if (test.IsPassed)
            {
                passed = passed + 1;
            }
            else
            {
                failed = failed + 1;
            }
        }
        Console.WriteLine("Барлығы: " + tests.Count);
        Console.WriteLine("Сәтті: " + passed);
        Console.WriteLine("Сәтсіз: " + failed);
        if (tests.Count > 0)
        {
            Console.WriteLine("Сәтті тесттер пайызы: " + (passed * 100.0 / tests.Count));
        }
        else
        {
            Console.WriteLine("Тесттер жоқ");
        }
    }
}
