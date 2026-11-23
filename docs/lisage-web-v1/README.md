# lisage : mise à jour de l'interface web v1

Contenu de l'archive, à remettre à l'agent Vibe Code du dépôt `saucisson/lisage` :

- `consigne-interface-web.md` : la consigne complète et autonome de mise à jour
  de l'interface web lisage (contrat commun zones / auth / Mistral / UI + travail
  spécifique au dépôt + issues + definition of done). À donner telle quelle.
- `logos/` : les fichiers de référence du logo lisage.
  - version claire : fond blanc, texte noir ;
  - version sombre : fond `#0F172A`, texte `#F1F5F9`, celle qu'affiche le
    sélecteur de thème en mode sombre ;
  - favicon : symbole seul, sans texte.

Intégration : déposer les fichiers logo dans les assets statiques du front
(l'emplacement figure dans la consigne), ne pas les retracer ni les redessiner ;
le sélecteur de thème bascule entre la version claire et la version sombre, le
favicon est déployé tel quel. Palette et règles communes : voir la consigne.
