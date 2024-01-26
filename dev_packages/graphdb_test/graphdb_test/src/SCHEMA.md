## Nodes
category
culture
event
meme
person
site
subculture
tag

image

## Edges
To define all possible relationships between the given nodes in a Neo4j graph database, we need to consider the logical connections that can exist between these entities. Each node represents a unique concept, such as a category, event, person, etc., and the relationships will capture how these concepts interact with each other.

I'll list out a range of potential relationships for each node type. This list is not exhaustive but covers a broad range of logical interactions:

Categories and Other Nodes:
- `BELONGS_TO`: Connects all nodes (culture, event, meme, etc.) to their respective categories.

Culture:
`INFLUENCES`: Culture to Event, Meme, Person, Site, Subculture (e.g., a music culture influencing a subculture).
`ORIGINATES_FROM`: Culture to Country (e.g., a food culture originating from a specific country).

Event:
`INVOLVES`: Event to Person, Organization (e.g., a person participating in an event).
`OCCURS_IN`: Event to Country, City (for location-specific events).
`CELEBRATES`: Event to Culture (e.g., a holiday celebrating a specific culture).
`FEATURES`: Event to Meme (e.g., a meme becoming popular during an event).

Meme:
`REFERENCES`: Meme to Culture, Event, Subculture (what the meme is about or refers to).
`ORIGINATED_ON`: site

Person:
`PARTICIPATES_IN`: Person to Event (e.g., an athlete in a sport event).
`CREATED`: Person to Meme, Culture (e.g., an artist creating a new art movement).
`BELONGS_TO`: Person to Subculture, Organization (e.g., a person being part of a subculture).
`INFLUENCES`: Person to Event, Culture, Meme (e.g., a politician influencing a political movement).

Site:
`HOSTS`: Site to Event, Meme (e.g., a social media site hosting a viral video).
`FOCUSES_ON`: Site to Culture, Subculture (e.g., a news site focusing on technology news).
`CREATED_BY`: Site to Person, Organization (who developed the site).

Subculture:
`DERIVED_FROM`: Subculture to Culture (e.g., a musical genre derived from a broader cultural movement).
`CELEBRATED_IN`: Subculture to Event (e.g., a subculture having its own specific events or conventions).
`POPULARIZED_BY`: Subculture to Person, Site (e.g., a subculture popularized by a celebrity or a website).

Generic Relationships:
`ASSOCIATED_WITH`: A generic relationship for when specific connections are not clear.
`CONTRIBUTES_TO`: For contributions that are not direct creation (e.g., a person contributing to a culture's spread).
`RELATED_TO`

## Origins
- youtube
- 4chan
- facebook
- tumblr
- reddit
- twitter
- tiktok


