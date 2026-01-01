use mazer_dbg::inspect;

fn main() {
    let mut ll = std::collections::LinkedList::new();
    ll.push_back("1");
    ll.push_front("2");
    ll.push_back("3");
    ll.push_front("4");

    let mut vec = vec![-1, -2, -3, -4];
    vec.push(-5);

    let mut map = std::collections::HashMap::new();
    map.insert("one", 1.0);
    map.insert("two", 2.0);
    map.insert("three", 3.0);
    map.insert("four", 4.0);

    let mut set = std::collections::HashSet::new();
    set.insert(10);
    set.insert(20);
    set.insert(30);
    set.insert(40);

    let mut bt = std::collections::BTreeMap::new();
    bt.insert("a", 1);
    bt.insert("b", 2);
    bt.insert("c", 3);
    bt.insert("d", 4);
    bt.insert("e", 5);

    inspect!(ll, vec, map, set, bt);
}
