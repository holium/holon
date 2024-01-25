import AirBnbMeme from "../assets/airbnb.jpg";
import CatMeme from "../assets/cat.png";
import BlimpMeme from "../assets/blimps.jpeg";
import WizardMeme from "../assets/wizard.jpeg";

export const memes = [CatMeme, WizardMeme, BlimpMeme, AirBnbMeme];

export type Category = {
  name: string;
  count: number;
};

export const categories: Category[] = [
  {
    name: "shizo",
    count: 121,
  },
  {
    name: "epstein",
    count: 34,
  },
  {
    name: "e/acc",
    count: 30,
  },
  {
    name: "decels",
    count: 5,
  },
  {
    name: "trump",
    count: 4,
  },
];

export type MemeTemplate = {
  name: string;
  count: number;
};

export const templates: MemeTemplate[] = [
  {
    name: "bell curve",
    count: 32,
  },
  {
    name: "distracted boyfriend",
    count: 12,
  },
  {
    name: "expanding brain",
    count: 16,
  },
  {
    name: "anakin padme 4 panel",
    count: 4,
  },
  {
    name: "two buttons",
    count: 3,
  },
];
