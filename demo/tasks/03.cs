using System;
using System.Collections.Generic;

abstract class GameCharacter
{
    public string Name { get; set; }
    public int Health { get; set; }
    public int Level { get; set; }
    public int Damage { get; set; }

    public abstract void Attack();
}

class Warrior : GameCharacter
{
    public string WeaponName { get; set; }

    public override void Attack()
    {
        Console.WriteLine(Name + " " + WeaponName + " каруымен шабуыл жасады, залал " + Damage);
    }
}

class Mage : GameCharacter
{
    public int MagicPower { get; set; }

    public override void Attack()
    {
        Console.WriteLine(Name + " сикырмен шабуыл жасады, куши " + MagicPower);
    }
}

class Archer : GameCharacter
{
    public int ArrowCount { get; set; }

    public override void Attack()
    {
        Console.WriteLine(Name + " жебемен шабуыл жасады, жебе саны " + ArrowCount);
    }
}

class Program
{
    static void Main()
    {
        List<GameCharacter> characters = new List<GameCharacter>();
        characters.Add(new Warrior { Name = "Аскар", Health = 120, Level = 5, Damage = 30, WeaponName = "семсер" });
        characters.Add(new Warrior { Name = "Даулет", Health = 40, Level = 3, Damage = 20, WeaponName = "айбалта" });
        characters.Add(new Mage { Name = "Алия", Health = 70, Level = 6, Damage = 45, MagicPower = 80 });
        characters.Add(new Mage { Name = "Ержан", Health = 0, Level = 2, Damage = 15, MagicPower = 30 });
        characters.Add(new Archer { Name = "Мадина", Health = 90, Level = 4, Damage = 25, ArrowCount = 20 });
        characters.Add(new Archer { Name = "Тимур", Health = 30, Level = 3, Damage = 18, ArrowCount = 12 });

        foreach (GameCharacter character in characters)
        {
            character.Attack();
        }

        Console.WriteLine("--- Денсаулыгы 50-ден томен кейіпкерлер ---");
        int alive = 0;
        foreach (GameCharacter character in characters)
        {
            if (character.Health < 50)
            {
                Console.WriteLine(character.Name + " - " + character.Health);
            }
            if (character.Health > 0)
            {
                alive = alive + 1;
            }
        }
        Console.WriteLine("Тірі кейіпкерлер саны: " + alive);
    }
}
