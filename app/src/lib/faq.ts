// The in-app FAQ. Plain words; every claim here is something the app really does.
export type Faq = { title: string; items: { q: string; a: string[] }[] }[];

export const FAQ: Faq = [
  {
    title: 'Getting started',
    items: [
      {
        q: 'What does VanityVault do?',
        a: [
          'It finds a Solana address that starts or ends with a word you choose, like one ending in “moon”. Every address has a private key, the secret that controls it. The app makes new keys on your own PC, keeps the one whose address matches and saves it in your vault, an encrypted file on your PC.',
          'On a good graphics card it tries millions of keys per second. Without a suitable graphics card it runs on the processor, which is much slower.',
        ],
      },
      {
        q: 'Who made it, and is it really free?',
        a: [
          'Ted K. Johansen, a developer in Norway. And yes, it’s free: the app and VanityCloud (the optional online backup) cost nothing. No ads, no tracking, and you don’t need an account or email to use the app.',
          'The code is open source on GitHub, so anyone can read it or build it themselves.',
        ],
      },
      {
        q: 'Does it touch my wallet or my money?',
        a: [
          'No. VanityVault never connects to Solana, can’t see balances and can’t send or sign anything. It never asks for your wallet or your recovery phrase and has nowhere to type one. Anyone who asks for your recovery phrase “for VanityVault” is a scammer.',
          'The app only goes online for VanityCloud, and only when you use it.',
        ],
      },
      {
        q: 'Is a vanity address as safe as a normal one?',
        a: [
          'Yes. It’s the same kind of key any wallet makes, from strong randomness. The app just throws keys away until one matches, and separate code double-checks every find.',
          'Some early vanity tools did get people robbed, because they took shortcuts with randomness. VanityVault takes none: every key comes fresh from Windows’ secure random source, and the code is open for anyone to check.',
        ],
      },
      {
        q: 'How do I install it, and why does Windows warn me?',
        a: [
          'Just run the installer. You need Windows 10 or 11, but no admin rights. VanityVault is new, so Windows may show “Windows protected your PC” for a while. Click “More info”, check that the publisher reads “Open Source Developer Ted Kenneth Johansen” (the certificate’s standard wording of the maker’s full name), then click “Run anyway”.',
          'If it says “Unknown publisher”, don’t run it. It’s not the real installer.',
        ],
      },
    ],
  },
  {
    title: 'Finding your word',
    items: [
      {
        q: 'How long will my word take?',
        a: [
          'The app shows an estimate before you start. As a rough guide on a fast graphics card, a word at the end of the address takes seconds with 5 characters, minutes with 6, hours with 7 and over a week with 8.',
          'The same word at the start takes about 17 times longer. Turning on “ignore upper/lower case” speeds things up, but the result may come out mixed, like “MooN”.',
        ],
      },
      {
        q: 'Which letters or numbers can’t I use?',
        a: [
          'Four: 0 (zero), O (capital o), I (capital i) and l (lowercase L). Solana addresses never contain them, and the app tells you if your word has one. With “ignore upper/lower case” on, those letters still work in your word: an l comes out as a capital L, and an o or i stays lowercase.',
        ],
      },
      {
        q: 'Can I hunt for several words at once?',
        a: ['Yes. The hunt list searches for up to 5000 words at once and saves each find to your vault as it comes in.'],
      },
      {
        q: 'Why does the app say it’s setting up my graphics card?',
        a: [
          'It’s a one-time setup of about 2 to 4 minutes, the first time you hunt after a new graphics driver (the software for your graphics card, which Windows updates now and then). You can hunt in the meantime: it starts on the processor and moves to the graphics card by itself.',
        ],
      },
      {
        q: 'Can I turn my current address into a vanity one?',
        a: ['No. A vanity address is always a brand-new address. To use it, move your funds to it like you would to any other address.'],
      },
    ],
  },
  {
    title: 'Your keys and the vault',
    items: [
      {
        q: 'What do I need to write down?',
        a: [
          'For the vault: your vault password and vault recovery key. If you use VanityCloud, also your username, VanityCloud password and VanityCloud recovery key.',
          'The app stores none of these, and nobody can reset them for you. Keep them somewhere safe, away from your PC.',
        ],
      },
      {
        q: 'Where are my keys kept?',
        a: [
          'In your vault: one encrypted file on your PC, locked with your vault password. When you create it, the app shows your vault recovery key once. Write it down or print it: it’s how you get back in if you forget your vault password.',
          'Your keys only ever leave your PC locked, and only if you use VanityCloud.',
        ],
      },
      {
        q: 'How do I use my new address in Phantom or Solflare?',
        a: [
          'In the Vault, choose “show secret key”, copy it and use “import private key” in your wallet. The app keeps the copied key out of Windows clipboard history and clears it from the clipboard after 30 seconds.',
          'Important: your wallet’s recovery phrase does not back up an imported key. Lose that device and the recovery phrase won’t bring this address back, so keep it in your vault or VanityCloud too.',
          'Anyone who sees this key can take everything in the address. Never paste it into a website or chat, never show it on a screen share and never give it to “support”.',
          'Developers: “save key file” puts a Solana keypair file in your Downloads folder. That file is a full copy of the key, so move it somewhere safe or delete it.',
        ],
      },
      {
        q: 'I forgot my vault password. What now?',
        a: [
          'Use your vault recovery key to set a new vault password. The app doesn’t store either one, so nobody can reset it for you, not even the maker. If you lose both, the keys in the vault are gone for good. Copies in VanityCloud or in a wallet aren’t affected.',
        ],
      },
      {
        q: 'What happens to my keys if my PC dies?',
        a: [
          'They’re safe only if a copy exists somewhere other than that PC: in VanityCloud, in a wallet on another device (like your phone), or in a key file you stored elsewhere. With no copy off the PC, the keys are gone, so set up a backup early.',
        ],
      },
      {
        q: 'Will uninstalling delete my keys?',
        a: ['No. Uninstalling keeps your vault, so if you reinstall, your keys are right where you left them.'],
      },
    ],
  },
  {
    title: 'The online backup (VanityCloud)',
    items: [
      {
        q: 'Can I get my keys on another PC?',
        a: [
          'Yes, with VanityCloud, the free and optional online backup. Install VanityVault on the other PC, sign in with your VanityCloud username and VanityCloud password (or VanityCloud recovery key), and restore.',
          'New finds aren’t uploaded by themselves unless you turn on “Back up new finds automatically” in the Backup tab; otherwise, back up again after you find something. Your VanityCloud password and VanityCloud recovery key are separate from your vault password and vault recovery key, so one never opens the other.',
        ],
      },
      {
        q: 'Can the maker or VanityCloud see my keys?',
        a: [
          'No. The app locks each key with your VanityCloud password on your PC before uploading it, and that password never leaves your PC. The VanityCloud server only gets locked boxes it can’t open.',
          'You don’t have to take anyone’s word for it: the code is open, the app shows you every message with your data in it, byte for byte, before sending it, and the website shows a real account exactly as the server stores it.',
        ],
      },
      {
        q: 'What if VanityCloud gets hacked or shuts down?',
        a: [
          'A hacker would get usernames and locked boxes, nothing readable. To open a box they’d have to guess your VanityCloud password, and the app makes every guess deliberately slow, which is why it insists on a strong one (don’t reuse a password from anywhere else). If anyone changes a box, the app refuses it, so nobody can slip you a fake key.',
          'And if VanityCloud ever shuts down, your keys are still in the vault on your PC. The backup is only a spare copy.',
        ],
      },
      {
        q: 'I forgot my VanityCloud password. What now?',
        a: [
          'Use your VanityCloud recovery key to set a new VanityCloud password. If you lose both, nobody can open the online copy, not even the maker. The vault on your PC isn’t affected.',
          'There’s no reset email either: VanityCloud never asks for your email (it won’t even accept one as a username), so write your username down too.',
        ],
      },
    ],
  },
  {
    title: 'Staying safe',
    items: [
      {
        q: 'Are there scams I should watch for?',
        a: [
          'Yes, lookalike addresses. Scammers create addresses that start and end like yours, betting you’ll only check your word, which is exactly what a vanity word tempts you to do. Always check the whole address before you send anything.',
        ],
      },
      {
        q: 'Why not just buy a vanity address?',
        a: [
          'Because whoever made it has seen its private key and could take anything you put in it, today or years from now. VanityVault makes the key on your own PC, so you’re the only one who ever sees it.',
        ],
      },
    ],
  },
];
