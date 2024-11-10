package pc.portal.pit.guest;
import org.teavm.interop.{Import,Export};
@Import(name="drop",module="tpit") @native def drop(handle: Int): Unit;

trait Handler[T]{
    type Impl <: T;
    def createImpl(a: T): Impl;
    def handleOf(a: Impl): Int;
    def fromHandle(a: Int): Impl;
    def finalize(a: T): Unit;
}
object Handler{
    given opt[T](using h: Handler[T]): Handler[Option[T]] = new Handler{
        type Impl = Option[h.Impl];
        def createImpl(a: Option[T]): Impl = a.map(h.createImpl);
        def handleOf(a: Impl): Int = a match{
            case None => 0
            case Some(value) => h.handleOf(value) + 1
        }
        ;
        def fromHandle(a: Int): Impl = if a == 0 then{
            None
        }else{
            Some(h.fromHandle(a - 1))
        }
        def finalize(a: Option[T]): Unit = a match{
            case None => ()
            case Some(value) => h.finalize(value)
        };
    };
    given int: Handler[Int] = new Handler[Int] {
        type Impl = Int;
        def createImpl(a: Int): Impl = a;
        def handleOf(a: Impl): Int = a;
        def fromHandle(a: Int): Impl = a;
        def finalize(a: Int): Unit = drop(a);
    }
}