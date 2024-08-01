function matches(tags, filter) {
  var hits = 0;
  if (filter == 'READ' || filter == 'UNREAD' || filter == 'READING') {
    if (tags.map((tag) => tag.toUpperCase()).includes(filter)) {
      hits += 10;
    }
  }


  for (let i = 0; i < tags.length; i++) {
    if (tags[i].toUpperCase().includes(filter)) {
      hits += 1;
    }
  }
  return hits;
}

function filter(list_id, query_id) {
  var input = document.getElementById(query_id);
  var filter = input.value.toUpperCase().split(/\s+/);
  console.log(filter)
  var ul = document.getElementById(list_id);
  li = ul.getElementsByTagName('li');

  if (filter == "") {
    for (let i = 0; i < li.length; ++i) {
      li[i].style.display = "";
      li[i].setAttribute('hits', 0);
    }
    return
  }

  var a;
  var elements = [];
  for (let i = 0; i < li.length; ++i) {
    a = li[i].getElementsByTagName('a')[0];
    var tags = li[i].getAttribute("tags").split(", ");
    var title = li[i].getAttribute("title").toUpperCase();
    var authors = li[i].getAttribute("authors").toUpperCase();
    var year = li[i].getAttribute("year");

    var hits = 0;
    for (const f of filter) {
      hits += matches(tags, f);
      for (t of title.split(/\s+/)) {
        if (t.includes(f)) hits += 1;
      }
      if (authors.includes(f)) hits += 1;
      if (year.includes(f)) hits += 1;
    }

    li[i].setAttribute('hits', hits);
    if (hits > 0.5 * filter.length) {
      li[i].style.display = "";
    } else {
      li[i].style.display = "none";
    }
  }

  Array.from(li).sort((a, b) => {
    var ka = a.getAttribute('hits');
    var kb = b.getAttribute('hits');
    if (ka < kb) return 1;
    if (ka > kb) return -1;
    return 0
  }).forEach(li => ul.appendChild(li));
}

function filter_tags(list_id, query_id) {
  var input = document.getElementById(query_id);
  var filter = input.value.toUpperCase();
  var ul = document.getElementById(list_id);
  li = ul.getElementsByTagName('li');

  for (let i = 0; i < li.length; ++i) {
    tag = li[i].textContent.toUpperCase();

    if (filter == "" || tag.includes(filter)) {
      li[i].style.display = "";
    } else {
      li[i].style.display = "none";
    }
  }
}
